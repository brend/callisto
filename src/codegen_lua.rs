use std::collections::HashMap;

use crate::{
    ast::Visibility,
    resolve::ResolvedModule,
    tir::{
        TirBinaryOp, TirBlock, TirExpr, TirExprKind, TirFunc, TirFuncKind, TirMatchArm, TirModule,
        TirPattern, TirPatternVariantPayload, TirStmt, TirUnaryOp, TirVariantPayload,
    },
    types::{FuncId, LocalId, VariantId},
};

pub fn emit_lua_module(tir: &TirModule, resolved: &ResolvedModule) -> String {
    let emitter = LuaEmitter::new(tir, resolved);
    emitter.emit()
}

struct LuaEmitter<'a> {
    tir: &'a TirModule,
    resolved: &'a ResolvedModule,
    out: String,
    indent: usize,
    variant_names: HashMap<VariantId, String>,
    func_lua_names: HashMap<FuncId, String>,
}

impl<'a> LuaEmitter<'a> {
    fn new(tir: &'a TirModule, resolved: &'a ResolvedModule) -> Self {
        let mut variant_names = HashMap::new();
        for type_info in &resolved.type_infos {
            if let crate::types::TypeKind::Sum(variants) = &type_info.kind {
                for variant in variants {
                    variant_names.insert(variant.id, variant.name.clone());
                }
            }
        }

        let mut func_lua_names = HashMap::new();
        for func in &tir.funcs {
            let lua_name = sanitize_ident(&func.name);
            func_lua_names.insert(func.id, lua_name);
        }

        Self {
            tir,
            resolved,
            out: String::new(),
            indent: 0,
            variant_names,
            func_lua_names,
        }
    }

    fn emit(mut self) -> String {
        self.line("local M = {}");
        self.line("");

        for func in &self.tir.funcs {
            if matches!(func.kind, TirFuncKind::Extern) {
                continue;
            }
            self.emit_function(func);
            self.line("");
        }

        for func in &self.tir.funcs {
            let info = &self.resolved.func_infos[func.id.0 as usize];
            if matches!(info.vis, Visibility::Public)
                && matches!(func.kind, TirFuncKind::Normal)
                && !info.name.contains('.')
            {
                let export_name = info.name.clone();
                let lua_name = self
                    .func_lua_names
                    .get(&func.id)
                    .cloned()
                    .unwrap_or_else(|| sanitize_ident(&func.name));
                self.line(&format!("M.{} = {}", export_name, lua_name));
            }
        }

        self.line("");
        self.line("return M");
        self.out
    }

    fn emit_function(&mut self, func: &TirFunc) {
        let lua_name = self
            .func_lua_names
            .get(&func.id)
            .cloned()
            .unwrap_or_else(|| sanitize_ident(&func.name));

        let params = func
            .params
            .iter()
            .map(|p| sanitize_ident(&p.name))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(&format!("local function {}({})", lua_name, params));
        self.indent += 1;

        let mut locals = HashMap::new();
        for p in &func.params {
            locals.insert(p.local, sanitize_ident(&p.name));
        }
        self.emit_block_statements(&func.body, &mut locals);

        self.indent -= 1;
        self.line("end");
    }

    fn emit_block_statements(&mut self, block: &TirBlock, locals: &mut HashMap<LocalId, String>) {
        for stmt in &block.stmts {
            self.emit_stmt(stmt, locals);
        }

        if let Some(tail) = &block.tail {
            let value = self.emit_expr(tail, locals);
            self.line(&format!("return {}", value));
        }
    }

    fn emit_stmt(&mut self, stmt: &TirStmt, locals: &mut HashMap<LocalId, String>) {
        match stmt {
            TirStmt::Let {
                local,
                value,
                mutable: _,
                ..
            } => {
                let local_name = format!("l{}", local.0);
                locals.insert(*local, local_name.clone());
                let value = self.emit_expr(value, locals);
                self.line(&format!("local {} = {}", local_name, value));
            }
            TirStmt::Assign { local, value } => {
                let name = locals
                    .get(local)
                    .cloned()
                    .unwrap_or_else(|| format!("l{}", local.0));
                let value = self.emit_expr(value, locals);
                self.line(&format!("{} = {}", name, value));
            }
            TirStmt::Expr(expr) => {
                let expr = self.emit_expr(expr, locals);
                self.line(&expr);
            }
            TirStmt::Return(value) => {
                if let Some(value) = value {
                    let expr = self.emit_expr(value, locals);
                    self.line(&format!("return {}", expr));
                } else {
                    self.line("return");
                }
            }
            TirStmt::While { cond, body } => {
                let cond = self.emit_expr(cond, locals);
                self.line(&format!("while {} do", cond));
                self.indent += 1;
                self.emit_block_statements(body, locals);
                self.indent -= 1;
                self.line("end");
            }
            TirStmt::ForRange {
                local,
                start,
                end,
                body,
            } => {
                let local_name = format!("l{}", local.0);
                locals.insert(*local, local_name.clone());
                let start = self.emit_expr(start, locals);
                let end = self.emit_expr(end, locals);
                self.line(&format!("for {} = {}, {} do", local_name, start, end));
                self.indent += 1;
                self.emit_block_statements(body, locals);
                self.indent -= 1;
                self.line("end");
            }
        }
    }

    fn emit_expr(&mut self, expr: &TirExpr, locals: &mut HashMap<LocalId, String>) -> String {
        match &expr.kind {
            TirExprKind::Int(v) => v.to_string(),
            TirExprKind::Float(v) => {
                if v.fract() == 0.0 {
                    format!("{:.1}", v)
                } else {
                    v.to_string()
                }
            }
            TirExprKind::String(s) => format!("{:?}", s),
            TirExprKind::Bool(v) => {
                if *v {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            TirExprKind::Unit => "nil".to_string(),
            TirExprKind::Local(local) => locals
                .get(local)
                .cloned()
                .unwrap_or_else(|| format!("l{}", local.0)),
            TirExprKind::Func(func_id) => self
                .func_lua_names
                .get(func_id)
                .cloned()
                .unwrap_or_else(|| format!("f{}", func_id.0)),
            TirExprKind::ExternPath(path) => path.join("."),
            TirExprKind::Call { callee, args } => {
                let callee = self.emit_expr(callee, locals);
                let args = args
                    .iter()
                    .map(|arg| self.emit_expr(arg, locals))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", callee, args)
            }
            TirExprKind::Field { base, field } => {
                let base = self.emit_expr(base, locals);
                format!("{}.{}", base, field)
            }
            TirExprKind::Binary { op, left, right } => {
                let op = lua_bin_op(*op);
                let left = self.emit_expr(left, locals);
                let right = self.emit_expr(right, locals);
                format!("({} {} {})", left, op, right)
            }
            TirExprKind::Unary { op, expr } => {
                let op = lua_unary_op(*op);
                let expr = self.emit_expr(expr, locals);
                format!("({} {})", op, expr)
            }
            TirExprKind::If {
                branches,
                else_branch,
            } => self.emit_if_expr(branches, else_branch, locals),
            TirExprKind::Match { scrutinee, arms } => self.emit_match_expr(scrutinee, arms, locals),
            TirExprKind::RecordInit { fields, .. } => {
                let fields = fields
                    .iter()
                    .map(|(name, value)| format!("{} = {}", name, self.emit_expr(value, locals)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields)
            }
            TirExprKind::RecordUpdate { base, fields, .. } => {
                let base_expr = self.emit_expr(base, locals);
                let mut out = String::new();
                out.push_str("(function(__base) ");
                out.push_str("local __tmp = {}; ");
                out.push_str("for k, v in pairs(__base) do __tmp[k] = v end; ");
                for (name, value) in fields {
                    out.push_str(&format!(
                        "__tmp.{} = {}; ",
                        name,
                        self.emit_expr(value, locals)
                    ));
                }
                out.push_str("return __tmp end)(");
                out.push_str(&base_expr);
                out.push(')');
                out
            }
            TirExprKind::VariantInit {
                variant_id,
                payload,
            } => self.emit_variant_init(*variant_id, payload, locals),
            TirExprKind::Lambda { params, body } => {
                let mut child_locals = locals.clone();
                let param_names = params
                    .iter()
                    .map(|(id, _)| {
                        let name = format!("l{}", id.0);
                        child_locals.insert(*id, name.clone());
                        name
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let body = self.emit_expr(body, &mut child_locals);
                format!("function({}) return {} end", param_names, body)
            }
        }
    }

    fn emit_if_expr(
        &mut self,
        branches: &[(TirExpr, TirBlock)],
        else_branch: &TirBlock,
        locals: &mut HashMap<LocalId, String>,
    ) -> String {
        let mut out = String::new();
        out.push_str("(function() ");
        for (idx, (cond, block)) in branches.iter().enumerate() {
            let cond = self.emit_expr(cond, locals);
            if idx == 0 {
                out.push_str(&format!("if {} then ", cond));
            } else {
                out.push_str(&format!("elseif {} then ", cond));
            }
            out.push_str(&self.emit_inline_block(block, locals));
            out.push(' ');
        }
        out.push_str("else ");
        out.push_str(&self.emit_inline_block(else_branch, locals));
        out.push_str(" end end)()");
        out
    }

    fn emit_match_expr(
        &mut self,
        scrutinee: &TirExpr,
        arms: &[TirMatchArm],
        locals: &mut HashMap<LocalId, String>,
    ) -> String {
        let scrutinee_expr = self.emit_expr(scrutinee, locals);
        let mut out = String::new();
        out.push_str("(function(__scrutinee) ");

        for (idx, arm) in arms.iter().enumerate() {
            let (cond, binds) =
                self.emit_pattern_cond_and_binds(&arm.pattern, "__scrutinee", locals);
            if idx == 0 {
                out.push_str(&format!("if {} then ", cond));
            } else {
                out.push_str(&format!("elseif {} then ", cond));
            }
            for bind in binds {
                out.push_str(&bind);
                out.push(' ');
            }
            out.push_str(&self.emit_inline_block(&arm.body, locals));
            out.push(' ');
        }
        out.push_str("else error(\"non-exhaustive match\") end end)(");
        out.push_str(&scrutinee_expr);
        out.push(')');
        out
    }

    fn emit_pattern_cond_and_binds(
        &self,
        pattern: &TirPattern,
        scrutinee_name: &str,
        locals: &HashMap<LocalId, String>,
    ) -> (String, Vec<String>) {
        match pattern {
            TirPattern::Wildcard => ("true".to_string(), Vec::new()),
            TirPattern::Bind(local) => {
                let name = locals
                    .get(local)
                    .cloned()
                    .unwrap_or_else(|| format!("l{}", local.0));
                (
                    "true".to_string(),
                    vec![format!("local {} = {}", name, scrutinee_name)],
                )
            }
            TirPattern::Int(v) => (format!("{} == {}", scrutinee_name, v), Vec::new()),
            TirPattern::Bool(v) => (
                format!(
                    "{} == {}",
                    scrutinee_name,
                    if *v { "true" } else { "false" }
                ),
                Vec::new(),
            ),
            TirPattern::String(v) => (format!("{} == {:?}", scrutinee_name, v), Vec::new()),
            TirPattern::Variant {
                variant_id,
                payload,
            } => {
                let tag = variant_id
                    .and_then(|id| self.variant_names.get(&id).cloned())
                    .unwrap_or_else(|| "<unknown>".to_string());
                let mut conds = vec![format!("{}.tag == {:?}", scrutinee_name, tag)];
                let mut binds = Vec::new();

                match payload {
                    TirPatternVariantPayload::None => {}
                    TirPatternVariantPayload::Positional(args) => {
                        for (idx, arg) in args.iter().enumerate() {
                            let value_name = format!("{}._{}", scrutinee_name, idx + 1);
                            let (arg_cond, arg_binds) =
                                self.emit_pattern_cond_and_binds(arg, &value_name, locals);
                            conds.push(arg_cond);
                            binds.extend(arg_binds);
                        }
                    }
                    TirPatternVariantPayload::Record(fields) => {
                        for (name, arg) in fields {
                            let value_name = format!("{}.{}", scrutinee_name, name);
                            let (arg_cond, arg_binds) =
                                self.emit_pattern_cond_and_binds(arg, &value_name, locals);
                            conds.push(arg_cond);
                            binds.extend(arg_binds);
                        }
                    }
                }

                (conds.join(" and "), binds)
            }
        }
    }

    fn emit_variant_init(
        &mut self,
        variant_id: VariantId,
        payload: &TirVariantPayload,
        locals: &mut HashMap<LocalId, String>,
    ) -> String {
        let tag = self
            .variant_names
            .get(&variant_id)
            .cloned()
            .unwrap_or_else(|| format!("Variant{}", variant_id.0));

        match payload {
            TirVariantPayload::None => format!("{{ tag = {:?} }}", tag),
            TirVariantPayload::Positional(values) => {
                let mut fields = vec![format!("tag = {:?}", tag)];
                for (idx, value) in values.iter().enumerate() {
                    fields.push(format!("_{} = {}", idx + 1, self.emit_expr(value, locals)));
                }
                format!("{{ {} }}", fields.join(", "))
            }
            TirVariantPayload::Record(values) => {
                let mut fields = vec![format!("tag = {:?}", tag)];
                for (name, value) in values {
                    fields.push(format!("{} = {}", name, self.emit_expr(value, locals)));
                }
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }

    fn emit_inline_block(
        &mut self,
        block: &TirBlock,
        locals: &mut HashMap<LocalId, String>,
    ) -> String {
        let mut out = String::new();
        for stmt in &block.stmts {
            out.push_str(&self.emit_inline_stmt(stmt, locals));
            out.push(' ');
        }
        if let Some(tail) = &block.tail {
            let expr = self.emit_expr(tail, locals);
            out.push_str(&format!("return {}", expr));
        } else {
            out.push_str("return nil");
        }
        out
    }

    fn emit_inline_stmt(
        &mut self,
        stmt: &TirStmt,
        locals: &mut HashMap<LocalId, String>,
    ) -> String {
        match stmt {
            TirStmt::Let { local, value, .. } => {
                let name = format!("l{}", local.0);
                locals.insert(*local, name.clone());
                let value = self.emit_expr(value, locals);
                format!("local {} = {};", name, value)
            }
            TirStmt::Assign { local, value } => {
                let name = locals
                    .get(local)
                    .cloned()
                    .unwrap_or_else(|| format!("l{}", local.0));
                let value = self.emit_expr(value, locals);
                format!("{} = {};", name, value)
            }
            TirStmt::Expr(expr) => {
                let expr = self.emit_expr(expr, locals);
                format!("{};", expr)
            }
            TirStmt::Return(value) => {
                if let Some(value) = value {
                    format!("return {};", self.emit_expr(value, locals))
                } else {
                    "return;".to_string()
                }
            }
            TirStmt::While { cond, body } => {
                let cond = self.emit_expr(cond, locals);
                let body = self.emit_inline_block(body, locals);
                format!("while {} do {} end;", cond, body)
            }
            TirStmt::ForRange {
                local,
                start,
                end,
                body,
            } => {
                let name = format!("l{}", local.0);
                locals.insert(*local, name.clone());
                let start = self.emit_expr(start, locals);
                let end = self.emit_expr(end, locals);
                let body = self.emit_inline_block(body, locals);
                format!("for {} = {}, {} do {} end;", name, start, end, body)
            }
        }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "v".to_string()
    } else if is_lua_keyword(&out) {
        format!("{}_", out)
    } else {
        out
    }
}

fn lua_bin_op(op: TirBinaryOp) -> &'static str {
    match op {
        TirBinaryOp::Or => "or",
        TirBinaryOp::And => "and",
        TirBinaryOp::Eq => "==",
        TirBinaryOp::NotEq => "~=",
        TirBinaryOp::Lt => "<",
        TirBinaryOp::LtEq => "<=",
        TirBinaryOp::Gt => ">",
        TirBinaryOp::GtEq => ">=",
        TirBinaryOp::Add => "+",
        TirBinaryOp::Sub => "-",
        TirBinaryOp::Mul => "*",
        TirBinaryOp::Div => "/",
        TirBinaryOp::Rem => "%",
    }
}

fn lua_unary_op(op: TirUnaryOp) -> &'static str {
    match op {
        TirUnaryOp::Neg => "-",
        TirUnaryOp::Not => "not",
    }
}

fn is_lua_keyword(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "break"
            | "do"
            | "else"
            | "elseif"
            | "end"
            | "false"
            | "for"
            | "function"
            | "if"
            | "in"
            | "local"
            | "nil"
            | "not"
            | "or"
            | "repeat"
            | "return"
            | "then"
            | "true"
            | "until"
            | "while"
    )
}
