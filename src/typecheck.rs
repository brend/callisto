use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        AssignStmt, BinaryOp, Block, ConstructorPayload, Expr, ExprKind, ForStmt, LetStmt,
        MatchArm, Param, Pattern, PatternKind, RecordPatternField, ReturnStmt, Stmt, TypeExpr,
        TypeExprKind, UnaryOp, VarStmt, WhileStmt,
    },
    diagnostics::Diagnostics,
    resolve::{ResolvedBody, ResolvedModule},
    tir::*,
    types::{
        FuncId, FuncKind, LocalId, Type, TypeId, TypeInfo, TypeKind, TypeParamId, VariantId,
        VariantPayload,
    },
};

pub fn typecheck_and_lower(resolved: &ResolvedModule) -> (TirModule, Diagnostics) {
    let mut checker = Checker::new(resolved);
    let tir = checker.run();
    (tir, checker.diagnostics)
}

struct Checker<'a> {
    resolved: &'a ResolvedModule,
    diagnostics: Diagnostics,
    next_local_id: u32,
    scopes: Vec<HashMap<String, LocalBinding>>,
    expected_ret: Type,
    current_type_params: HashMap<String, TypeParamId>,
}

#[derive(Debug, Clone)]
struct LocalBinding {
    id: LocalId,
    ty: Type,
    mutable: bool,
}

impl<'a> Checker<'a> {
    fn new(resolved: &'a ResolvedModule) -> Self {
        Self {
            resolved,
            diagnostics: Diagnostics::new(),
            next_local_id: 0,
            scopes: Vec::new(),
            expected_ret: Type::Unit,
            current_type_params: HashMap::new(),
        }
    }

    fn run(&mut self) -> TirModule {
        let mut funcs_by_id: Vec<Option<TirFunc>> = vec![None; self.resolved.func_infos.len()];

        for body in &self.resolved.bodies {
            let tir_func = self.check_func_body(body);
            funcs_by_id[body.func_id.0 as usize] = Some(tir_func);
        }

        for (id, info) in self.resolved.func_infos.iter().enumerate() {
            if funcs_by_id[id].is_none() {
                let kind = self.lower_func_kind(&info.kind);
                funcs_by_id[id] = Some(TirFunc {
                    id: FuncId(id as u32),
                    name: info.name.clone(),
                    params: Vec::new(),
                    ret_ty: info.ret.clone(),
                    body: TirBlock {
                        stmts: Vec::new(),
                        tail: None,
                    },
                    kind,
                });
            }
        }

        let funcs = funcs_by_id.into_iter().flatten().collect();
        let types = self
            .resolved
            .type_infos
            .iter()
            .enumerate()
            .map(|(id, t)| TirTypeDecl {
                id: TypeId(id as u32),
                name: t.name.clone(),
            })
            .collect();

        TirModule { types, funcs }
    }

    fn check_func_body(&mut self, body: &ResolvedBody) -> TirFunc {
        let func_info = &self.resolved.func_infos[body.func_id.0 as usize];
        self.expected_ret = func_info.ret.clone();
        self.current_type_params.clear();
        for (name, id) in body.decl.type_params.iter().zip(&func_info.type_params) {
            self.current_type_params.insert(name.clone(), *id);
        }

        self.scopes.clear();
        self.push_scope();

        let mut tir_params = Vec::new();
        for (idx, param) in body.decl.params.iter().enumerate() {
            let ty = func_info.params.get(idx).cloned().unwrap_or(Type::Error);
            let local = self.alloc_local(param.name.clone(), ty.clone(), false, param.span);
            tir_params.push(TirParam {
                local,
                name: param.name.clone(),
                ty,
            });
        }

        let body_block = self.check_block(&body.decl.body);
        self.pop_scope();

        TirFunc {
            id: body.func_id,
            name: func_info.name.clone(),
            params: tir_params,
            ret_ty: func_info.ret.clone(),
            body: body_block,
            kind: self.lower_func_kind(&func_info.kind),
        }
    }

    fn check_block(&mut self, block: &Block) -> TirBlock {
        self.push_scope();

        let mut stmts = Vec::new();
        for stmt in &block.stmts {
            if let Some(tir_stmt) = self.check_stmt(stmt) {
                stmts.push(tir_stmt);
            }
        }

        let expected_tail = self.expected_ret.clone();
        let tail = block
            .tail
            .as_ref()
            .map(|expr| Box::new(self.check_expr_with_expected(expr, Some(&expected_tail))));
        if let Some(tail_expr) = &tail {
            if !self.is_assignable(&self.expected_ret, &tail_expr.ty)
                && !matches!(self.normalize_type(&self.expected_ret), Type::Unit)
            {
                self.diagnostics.error(
                    block.span,
                    format!(
                        "tail expression has type {:?} but function expects {:?}",
                        tail_expr.ty, self.expected_ret
                    ),
                );
            }
        }

        self.pop_scope();
        TirBlock { stmts, tail }
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Option<TirStmt> {
        Some(match stmt {
            Stmt::Let(s) => self.check_let_stmt(s),
            Stmt::Var(s) => self.check_var_stmt(s),
            Stmt::Assign(s) => self.check_assign_stmt(s),
            Stmt::Expr(s) => TirStmt::Expr(self.check_expr(&s.expr)),
            Stmt::Return(s) => self.check_return_stmt(s),
            Stmt::While(s) => self.check_while_stmt(s),
            Stmt::For(s) => self.check_for_stmt(s),
        })
    }

    fn check_let_stmt(&mut self, stmt: &LetStmt) -> TirStmt {
        let annotation = stmt.ty.as_ref().map(|ann| self.lower_type_expr(ann, false));
        let value = if let Some(ann_ty) = &annotation {
            self.check_expr_with_expected(&stmt.value, Some(ann_ty))
        } else {
            self.check_expr(&stmt.value)
        };
        let ty = if let Some(ann_ty) = annotation {
            if !self.is_assignable(&ann_ty, &value.ty) {
                self.diagnostics.error(
                    stmt.span,
                    format!(
                        "let binding '{}' has type {:?} but annotation is {:?}",
                        stmt.name, value.ty, ann_ty
                    ),
                );
            }
            ann_ty
        } else {
            value.ty.clone()
        };
        let local = self.alloc_local(stmt.name.clone(), ty.clone(), false, stmt.span);
        TirStmt::Let {
            local,
            ty,
            value,
            mutable: false,
        }
    }

    fn check_var_stmt(&mut self, stmt: &VarStmt) -> TirStmt {
        let annotation = stmt.ty.as_ref().map(|ann| self.lower_type_expr(ann, false));
        let value = if let Some(ann_ty) = &annotation {
            self.check_expr_with_expected(&stmt.value, Some(ann_ty))
        } else {
            self.check_expr(&stmt.value)
        };
        let ty = if let Some(ann_ty) = annotation {
            if !self.is_assignable(&ann_ty, &value.ty) {
                self.diagnostics.error(
                    stmt.span,
                    format!(
                        "var binding '{}' has type {:?} but annotation is {:?}",
                        stmt.name, value.ty, ann_ty
                    ),
                );
            }
            ann_ty
        } else {
            value.ty.clone()
        };
        let local = self.alloc_local(stmt.name.clone(), ty.clone(), true, stmt.span);
        TirStmt::Let {
            local,
            ty,
            value,
            mutable: true,
        }
    }

    fn check_assign_stmt(&mut self, stmt: &AssignStmt) -> TirStmt {
        let value = self.check_expr(&stmt.value);
        let Some(binding) = self.lookup_local(&stmt.target).cloned() else {
            self.diagnostics.error(
                stmt.span,
                format!(
                    "assignment target '{}' is not a local variable",
                    stmt.target
                ),
            );
            return TirStmt::Expr(value);
        };
        if !binding.mutable {
            self.diagnostics.error(
                stmt.span,
                format!("cannot assign to immutable local '{}'", stmt.target),
            );
        }
        if !self.is_assignable(&binding.ty, &value.ty) {
            self.diagnostics.error(
                stmt.span,
                format!(
                    "cannot assign value of type {:?} to local '{}' of type {:?}",
                    value.ty, stmt.target, binding.ty
                ),
            );
        }

        TirStmt::Assign {
            local: binding.id,
            value,
        }
    }

    fn check_return_stmt(&mut self, stmt: &ReturnStmt) -> TirStmt {
        let expected = self.expected_ret.clone();
        let value = stmt
            .value
            .as_ref()
            .map(|v| self.check_expr_with_expected(v, Some(&expected)));
        let ret_ty = value.as_ref().map(|v| v.ty.clone()).unwrap_or(Type::Unit);
        if !self.is_assignable(&self.expected_ret, &ret_ty) {
            self.diagnostics.error(
                stmt.span,
                format!(
                    "return type {:?} does not match expected {:?}",
                    ret_ty, self.expected_ret
                ),
            );
        }
        TirStmt::Return(value)
    }

    fn check_while_stmt(&mut self, stmt: &WhileStmt) -> TirStmt {
        let cond = self.check_expr(&stmt.cond);
        if !self.is_bool_type(&cond.ty) {
            self.diagnostics
                .error(stmt.cond.span, "while condition must be Bool");
        }
        let body = self.check_block(&stmt.body);
        TirStmt::While { cond, body }
    }

    fn check_for_stmt(&mut self, stmt: &ForStmt) -> TirStmt {
        let start = self.check_expr(&stmt.start);
        let end = self.check_expr(&stmt.end);
        if !self.is_int_type(&start.ty) || !self.is_int_type(&end.ty) {
            self.diagnostics
                .error(stmt.span, "for range bounds must be Int expressions");
        }

        self.push_scope();
        let local = self.alloc_local(stmt.name.clone(), Type::Int, false, stmt.span);
        let body = self.check_block(&stmt.body);
        self.pop_scope();

        TirStmt::ForRange {
            local,
            start,
            end,
            body,
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> TirExpr {
        self.check_expr_with_expected(expr, None)
    }

    fn check_expr_with_expected(&mut self, expr: &Expr, expected: Option<&Type>) -> TirExpr {
        match &expr.kind {
            ExprKind::Int(v) => self.mk_expr(Type::Int, TirExprKind::Int(*v)),
            ExprKind::Float(v) => self.mk_expr(Type::Float, TirExprKind::Float(*v)),
            ExprKind::String(v) => self.mk_expr(Type::String, TirExprKind::String(v.clone())),
            ExprKind::Bool(v) => self.mk_expr(Type::Bool, TirExprKind::Bool(*v)),
            ExprKind::Unit => self.mk_expr(Type::Unit, TirExprKind::Unit),
            ExprKind::Paren(inner) => self.check_expr_with_expected(inner, expected),
            ExprKind::Var(name) => self.check_var_expr(name, expr.span, expected),
            ExprKind::Path(path) => self.check_path_expr(path, expr.span),
            ExprKind::Call { callee, args } => self.check_call_expr(callee, args, expr.span),
            ExprKind::Field { receiver, name } => self.check_field_expr(receiver, name, expr.span),
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => self.check_method_call_expr(receiver, method, args, expr.span),
            ExprKind::Binary { op, left, right } => {
                self.check_binary_expr(*op, left, right, expr.span)
            }
            ExprKind::Unary { op, expr } => self.check_unary_expr(*op, expr, expr.span),
            ExprKind::If {
                branches,
                else_branch,
            } => self.check_if_expr(branches, else_branch),
            ExprKind::Match { scrutinee, arms } => self.check_match_expr(scrutinee, arms),
            ExprKind::RecordInit { type_name, fields } => {
                self.check_record_init_expr(type_name, fields, expr.span, expected)
            }
            ExprKind::RecordUpdate { base, fields } => {
                self.check_record_update_expr(base, fields, expr.span)
            }
            ExprKind::Constructor { name, payload } => {
                self.check_constructor_expr(name, payload, expr.span, expected)
            }
            ExprKind::Lambda {
                params,
                ret_ty,
                body,
            } => self.check_lambda_expr(params, ret_ty, body),
        }
    }

    fn check_var_expr(
        &mut self,
        name: &str,
        span: crate::span::Span,
        expected: Option<&Type>,
    ) -> TirExpr {
        if let Some(local) = self.lookup_local(name).cloned() {
            return self.mk_expr(local.ty, TirExprKind::Local(local.id));
        }
        if let Some(qualified) = self.resolved.import_items.get(name) {
            if let Some(func_id) = self.resolved.func_names.get(qualified).copied() {
                let func_info = &self.resolved.func_infos[func_id.0 as usize];
                return self.mk_expr(
                    Type::Func(func_info.params.clone(), Box::new(func_info.ret.clone())),
                    TirExprKind::Func(func_id),
                );
            }
            self.diagnostics.error(
                span,
                format!(
                    "imported item '{}' resolves to '{}' but no matching function/extern declaration exists",
                    name, qualified
                ),
            );
            return self.mk_expr(
                Type::Error,
                TirExprKind::ExternPath(qualified.split('.').map(ToString::to_string).collect()),
            );
        }
        if let Some(func_id) = self.resolved.func_names.get(name).copied() {
            let func_info = &self.resolved.func_infos[func_id.0 as usize];
            return self.mk_expr(
                Type::Func(func_info.params.clone(), Box::new(func_info.ret.clone())),
                TirExprKind::Func(func_id),
            );
        }
        if let Some(variant_id) = self.resolved.variant_names.get(name).copied() {
            if let Some(ty_id) = self.resolved.variant_to_type.get(&variant_id).copied() {
                let inferred_args = self
                    .expected_named_type_args(expected, ty_id)
                    .unwrap_or_else(|| self.default_type_args_for_type(ty_id));
                let named = Type::Named(ty_id, inferred_args);
                let payload = self.variant_payload(variant_id);
                if matches!(payload, Some(VariantPayload::None)) {
                    if self.expected_named_type_args(expected, ty_id).is_none()
                        && self
                            .resolved
                            .type_infos
                            .get(ty_id.0 as usize)
                            .is_some_and(|info| !info.params.is_empty())
                    {
                        self.diagnostics.error(
                            span,
                            format!(
                                "cannot infer generic type arguments for constructor '{}' without context",
                                name
                            ),
                        );
                    }
                    return self.mk_expr(
                        named,
                        TirExprKind::VariantInit {
                            variant_id,
                            payload: TirVariantPayload::None,
                        },
                    );
                }
                // Constructor with payload: represent as extern path fallback until called.
                return self.mk_expr(Type::Error, TirExprKind::ExternPath(vec![name.to_string()]));
            }
        }
        if let Some(path) = self.resolved.import_modules.get(name) {
            return self.mk_expr(Type::Error, TirExprKind::ExternPath(path.clone()));
        }

        self.diagnostics
            .error(span, format!("unresolved name '{}'", name));
        self.mk_expr(Type::Error, TirExprKind::ExternPath(vec![name.to_string()]))
    }

    fn check_path_expr(&mut self, path: &[String], span: crate::span::Span) -> TirExpr {
        let rewritten = self.rewrite_import_path(path);
        let name = rewritten.join(".");
        if let Some(func_id) = self.resolved.func_names.get(&name).copied() {
            let func_info = &self.resolved.func_infos[func_id.0 as usize];
            return self.mk_expr(
                Type::Func(func_info.params.clone(), Box::new(func_info.ret.clone())),
                TirExprKind::Func(func_id),
            );
        }
        if self.path_references_import_alias(path) {
            self.diagnostics.error(
                span,
                format!(
                    "imported path '{}' has no matching function/extern declaration",
                    name
                ),
            );
        }
        self.mk_expr(Type::Error, TirExprKind::ExternPath(rewritten))
    }

    fn check_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: crate::span::Span,
    ) -> TirExpr {
        let callee_tir = self.check_expr(callee);
        let hint_params = match &callee_tir.ty {
            Type::Func(params, _) => Some(params.clone()),
            _ => None,
        };
        let args_tir: Vec<TirExpr> = args
            .iter()
            .enumerate()
            .map(|(idx, arg)| {
                let hint = hint_params.as_ref().and_then(|params| params.get(idx));
                self.check_expr_with_expected(arg, hint)
            })
            .collect();

        match &callee_tir.ty {
            Type::Func(params, ret) => {
                let (effective_params, effective_ret) =
                    if let TirExprKind::Func(func_id) = &callee_tir.kind {
                        self.instantiate_func_signature(*func_id, &args_tir, span)
                    } else {
                        (params.clone(), (*ret.clone()).clone())
                    };
                self.check_call_args(span, &effective_params, &args_tir);
                self.mk_expr(
                    effective_ret,
                    TirExprKind::Call {
                        callee: Box::new(callee_tir),
                        args: args_tir,
                    },
                )
            }
            Type::Error => self.mk_expr(
                Type::Error,
                TirExprKind::Call {
                    callee: Box::new(callee_tir),
                    args: args_tir,
                },
            ),
            _ => {
                self.diagnostics
                    .error(span, "attempted to call a non-function value");
                self.mk_expr(
                    Type::Error,
                    TirExprKind::Call {
                        callee: Box::new(callee_tir),
                        args: args_tir,
                    },
                )
            }
        }
    }

    fn check_field_expr(
        &mut self,
        receiver: &Expr,
        name: &str,
        span: crate::span::Span,
    ) -> TirExpr {
        let base = self.check_expr(receiver);
        if let TirExprKind::ExternPath(segments) = &base.kind {
            let mut next = segments.clone();
            next.push(name.to_string());
            let joined = next.join(".");
            if let Some(func_id) = self.resolved.func_names.get(&joined).copied() {
                let func_info = &self.resolved.func_infos[func_id.0 as usize];
                return self.mk_expr(
                    Type::Func(func_info.params.clone(), Box::new(func_info.ret.clone())),
                    TirExprKind::Func(func_id),
                );
            }
            if self.is_imported_module_path(segments) {
                self.diagnostics.error(
                    span,
                    format!(
                        "unknown imported module member '{}'; add a matching extern declaration",
                        joined
                    ),
                );
            }
            return self.mk_expr(Type::Error, TirExprKind::ExternPath(next));
        }

        if let Type::Named(type_id, type_args) = &base.ty {
            if let Some(field_ty) = self.lookup_record_field(*type_id, type_args, name) {
                return self.mk_expr(
                    field_ty,
                    TirExprKind::Field {
                        base: Box::new(base),
                        field: name.to_string(),
                    },
                );
            }
        }

        self.diagnostics
            .error(span, format!("cannot access field '{}'", name));
        self.mk_expr(
            Type::Error,
            TirExprKind::Field {
                base: Box::new(base),
                field: name.to_string(),
            },
        )
    }

    fn check_method_call_expr(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
        span: crate::span::Span,
    ) -> TirExpr {
        let recv = self.check_expr(receiver);

        if let TirExprKind::ExternPath(segments) = &recv.kind {
            let mut next = segments.clone();
            next.push(method.to_string());
            let joined = next.join(".");
            if let Some(func_id) = self.resolved.func_names.get(&joined).copied() {
                let param_hints = self.resolved.func_infos[func_id.0 as usize].params.clone();
                let args_tir: Vec<TirExpr> = args
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.check_expr_with_expected(arg, param_hints.get(idx)))
                    .collect();
                let (effective_params, effective_ret) =
                    self.instantiate_func_signature(func_id, &args_tir, span);
                self.check_call_args(span, &effective_params, &args_tir);
                let callee = self.mk_expr(
                    Type::Func(effective_params, Box::new(effective_ret.clone())),
                    TirExprKind::Func(func_id),
                );
                return self.mk_expr(
                    effective_ret,
                    TirExprKind::Call {
                        callee: Box::new(callee),
                        args: args_tir,
                    },
                );
            }
            if self.is_imported_module_path(segments) {
                self.diagnostics.error(
                    span,
                    format!(
                        "unknown imported module function '{}'; add a matching extern declaration",
                        joined
                    ),
                );
            }
            let callee = TirExpr {
                ty: Type::Error,
                kind: TirExprKind::ExternPath(next),
            };
            let args_tir = args
                .iter()
                .map(|a| self.check_expr_with_expected(a, None))
                .collect();
            return self.mk_expr(
                Type::Error,
                TirExprKind::Call {
                    callee: Box::new(callee),
                    args: args_tir,
                },
            );
        }

        let Some((type_id, _)) = self.normalized_named_type(&recv.ty) else {
            self.diagnostics
                .error(span, format!("unknown method '{}'", method));
            return self.mk_expr(Type::Error, TirExprKind::Unit);
        };

        let Some(func_id) = self
            .resolved
            .method_names
            .get(&(type_id, method.to_string()))
            .copied()
        else {
            self.diagnostics.error(
                span,
                format!("unknown method '{}' for receiver type", method),
            );
            return self.mk_expr(Type::Error, TirExprKind::Unit);
        };

        let method_param_hints = self.resolved.func_infos[func_id.0 as usize].params.clone();
        let mut args_tir = Vec::new();
        args_tir.push(recv);
        args_tir.extend(
            args.iter().enumerate().map(|(idx, arg)| {
                self.check_expr_with_expected(arg, method_param_hints.get(idx + 1))
            }),
        );
        let (effective_params, effective_ret) =
            self.instantiate_func_signature(func_id, &args_tir, span);
        self.check_call_args(span, &effective_params, &args_tir);

        let callee = self.mk_expr(
            Type::Func(effective_params, Box::new(effective_ret.clone())),
            TirExprKind::Func(func_id),
        );
        self.mk_expr(
            effective_ret,
            TirExprKind::Call {
                callee: Box::new(callee),
                args: args_tir,
            },
        )
    }

    fn check_binary_expr(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: crate::span::Span,
    ) -> TirExpr {
        let left = self.check_expr(left);
        let right = self.check_expr(right);

        let (result_ty, tir_op) = match op {
            BinaryOp::Or | BinaryOp::And => {
                if !self.is_bool_type(&left.ty) || !self.is_bool_type(&right.ty) {
                    self.diagnostics
                        .error(span, "logical operators require Bool operands");
                }
                (
                    Type::Bool,
                    if op == BinaryOp::Or {
                        TirBinaryOp::Or
                    } else {
                        TirBinaryOp::And
                    },
                )
            }
            BinaryOp::Eq | BinaryOp::NotEq => {
                if !self.is_assignable(&left.ty, &right.ty)
                    && !self.is_assignable(&right.ty, &left.ty)
                {
                    self.diagnostics
                        .error(span, "equality operands must have compatible types");
                }
                (
                    Type::Bool,
                    if op == BinaryOp::Eq {
                        TirBinaryOp::Eq
                    } else {
                        TirBinaryOp::NotEq
                    },
                )
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if !self.is_numeric_type(&left.ty) || !self.is_numeric_type(&right.ty) {
                    self.diagnostics
                        .error(span, "comparison operators require numeric operands");
                }
                (
                    Type::Bool,
                    match op {
                        BinaryOp::Lt => TirBinaryOp::Lt,
                        BinaryOp::LtEq => TirBinaryOp::LtEq,
                        BinaryOp::Gt => TirBinaryOp::Gt,
                        BinaryOp::GtEq => TirBinaryOp::GtEq,
                        _ => unreachable!(),
                    },
                )
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                let left_norm = self.normalize_type(&left.ty);
                let right_norm = self.normalize_type(&right.ty);
                if !is_numeric(&left_norm) || !is_numeric(&right_norm) {
                    self.diagnostics
                        .error(span, "arithmetic operators require numeric operands");
                    (Type::Error, TirBinaryOp::Add)
                } else if matches!(left_norm, Type::Float) || matches!(right_norm, Type::Float) {
                    (
                        Type::Float,
                        match op {
                            BinaryOp::Add => TirBinaryOp::Add,
                            BinaryOp::Sub => TirBinaryOp::Sub,
                            BinaryOp::Mul => TirBinaryOp::Mul,
                            BinaryOp::Div => TirBinaryOp::Div,
                            BinaryOp::Rem => TirBinaryOp::Rem,
                            _ => unreachable!(),
                        },
                    )
                } else {
                    (
                        Type::Int,
                        match op {
                            BinaryOp::Add => TirBinaryOp::Add,
                            BinaryOp::Sub => TirBinaryOp::Sub,
                            BinaryOp::Mul => TirBinaryOp::Mul,
                            BinaryOp::Div => TirBinaryOp::Div,
                            BinaryOp::Rem => TirBinaryOp::Rem,
                            _ => unreachable!(),
                        },
                    )
                }
            }
        };

        self.mk_expr(
            result_ty,
            TirExprKind::Binary {
                op: tir_op,
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    fn check_unary_expr(&mut self, op: UnaryOp, expr: &Expr, span: crate::span::Span) -> TirExpr {
        let inner = self.check_expr(expr);
        match op {
            UnaryOp::Neg => {
                if !self.is_numeric_type(&inner.ty) {
                    self.diagnostics
                        .error(span, "negation requires a numeric operand");
                }
                self.mk_expr(
                    inner.ty.clone(),
                    TirExprKind::Unary {
                        op: TirUnaryOp::Neg,
                        expr: Box::new(inner),
                    },
                )
            }
            UnaryOp::Not => {
                if !self.is_bool_type(&inner.ty) {
                    self.diagnostics
                        .error(span, "logical not requires a Bool operand");
                }
                self.mk_expr(
                    Type::Bool,
                    TirExprKind::Unary {
                        op: TirUnaryOp::Not,
                        expr: Box::new(inner),
                    },
                )
            }
        }
    }

    fn check_if_expr(&mut self, branches: &[(Expr, Block)], else_branch: &Block) -> TirExpr {
        let mut tir_branches = Vec::new();
        let mut result_ty: Option<Type> = None;

        for (cond, body) in branches {
            let cond_tir = self.check_expr(cond);
            if !self.is_bool_type(&cond_tir.ty) {
                self.diagnostics
                    .error(cond.span, "if condition must have type Bool");
            }
            let body_tir = self.check_block(body);
            if let Some(tail) = &body_tir.tail {
                result_ty = Some(self.unify_branch_type(
                    result_ty,
                    tail.ty.clone(),
                    body.span,
                    "if branch",
                ));
            }
            tir_branches.push((cond_tir, body_tir));
        }

        let else_tir = self.check_block(else_branch);
        if let Some(tail) = &else_tir.tail {
            result_ty = Some(self.unify_branch_type(
                result_ty,
                tail.ty.clone(),
                else_branch.span,
                "if branch",
            ));
        }

        let final_ty = result_ty.unwrap_or(Type::Unit);
        self.mk_expr(
            final_ty,
            TirExprKind::If {
                branches: tir_branches,
                else_branch: Box::new(else_tir),
            },
        )
    }

    fn check_match_expr(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> TirExpr {
        let scrutinee_tir = self.check_expr(scrutinee);
        let mut tir_arms = Vec::new();
        let mut result_ty: Option<Type> = None;
        let mut has_catch_all = false;
        let mut covered_variants = HashSet::new();

        for arm in arms {
            self.push_scope();
            let pattern = self.check_pattern(&arm.pattern, &scrutinee_tir.ty);
            if matches!(pattern, TirPattern::Wildcard | TirPattern::Bind(_)) {
                has_catch_all = true;
            }
            if let TirPattern::Variant {
                variant_id: Some(variant_id),
                ..
            } = &pattern
            {
                covered_variants.insert(*variant_id);
            }
            let body = self.check_block(&arm.body);
            if let Some(tail) = &body.tail {
                result_ty = Some(self.unify_branch_type(
                    result_ty,
                    tail.ty.clone(),
                    arm.body.span,
                    "match arm",
                ));
            }
            self.pop_scope();
            tir_arms.push(TirMatchArm { pattern, body });
        }

        if let Some(sum_variants) = self.sum_variants_for_type(&scrutinee_tir.ty) {
            if !has_catch_all {
                let missing: Vec<String> = sum_variants
                    .iter()
                    .filter(|v| !covered_variants.contains(&v.id))
                    .map(|v| v.name.clone())
                    .collect();
                if !missing.is_empty() {
                    self.diagnostics.error(
                        scrutinee.span,
                        format!(
                            "non-exhaustive match, missing variants: {}",
                            missing.join(", ")
                        ),
                    );
                }
            }
        }

        self.mk_expr(
            result_ty.unwrap_or(Type::Unit),
            TirExprKind::Match {
                scrutinee: Box::new(scrutinee_tir),
                arms: tir_arms,
            },
        )
    }

    fn check_record_init_expr(
        &mut self,
        type_name: &str,
        fields: &[crate::ast::RecordFieldInit],
        span: crate::span::Span,
        expected: Option<&Type>,
    ) -> TirExpr {
        if let Some(variant_id) = self.resolved.variant_names.get(type_name).copied() {
            let payload = TirVariantPayload::Record(
                fields
                    .iter()
                    .map(|f| (f.name.clone(), self.check_expr(&f.value)))
                    .collect(),
            );
            if let Some(ty_id) = self.resolved.variant_to_type.get(&variant_id).copied() {
                if let Some((_, expected_payload_raw)) = self.variant_info(variant_id) {
                    let mut subst: HashMap<TypeParamId, Type> = HashMap::new();
                    infer_variant_payload_type_params(&expected_payload_raw, &payload, &mut subst);
                    let context = format!("constructor '{}'", type_name);
                    let expected_args = self.expected_named_type_args(expected, ty_id);
                    let type_args = self.finalize_inferred_type_args(
                        ty_id,
                        &mut subst,
                        span,
                        &context,
                        expected_args,
                    );
                    let expected_payload =
                        self.instantiate_variant_payload(&expected_payload_raw, &subst);
                    self.check_constructor_payload(&expected_payload, &payload, span);

                    return self.mk_expr(
                        Type::Named(ty_id, type_args),
                        TirExprKind::VariantInit {
                            variant_id,
                            payload,
                        },
                    );
                }

                let fallback_args = self
                    .expected_named_type_args(expected, ty_id)
                    .unwrap_or_else(|| self.default_type_args_for_type(ty_id));
                return self.mk_expr(
                    Type::Named(ty_id, fallback_args),
                    TirExprKind::VariantInit {
                        variant_id,
                        payload,
                    },
                );
            }
        }

        let Some(type_id) = self.resolved.type_names.get(type_name).copied() else {
            self.diagnostics
                .error(span, format!("unknown type '{}'", type_name));
            return self.mk_expr(Type::Error, TirExprKind::Unit);
        };

        let record_fields: Vec<(String, TirExpr)> = fields
            .iter()
            .map(|f| (f.name.clone(), self.check_expr(&f.value)))
            .collect();
        let mut type_args = self.default_type_args_for_type(type_id);

        if let Some(TypeInfo {
            kind: TypeKind::Record(expected_fields),
            ..
        }) = self.resolved.type_infos.get(type_id.0 as usize)
        {
            let mut subst: HashMap<TypeParamId, Type> = HashMap::new();
            infer_record_field_type_params(expected_fields, &record_fields, &mut subst);
            let context = format!("record initializer '{}'", type_name);
            let expected_args = self.expected_named_type_args(expected, type_id);
            type_args = self.finalize_inferred_type_args(
                type_id,
                &mut subst,
                span,
                &context,
                expected_args,
            );
            let expected_fields = self.instantiate_record_fields(expected_fields, &subst);

            let provided: Vec<(String, crate::span::Span)> =
                fields.iter().map(|f| (f.name.clone(), f.span)).collect();
            self.validate_record_field_set(span, &expected_fields, &provided, "record initializer");
            for (name, value) in &record_fields {
                if let Some(expected) = expected_fields.iter().find(|f| f.name == name.as_str()) {
                    if !self.is_assignable(&expected.ty, &value.ty) {
                        self.diagnostics.error(
                            span,
                            format!(
                                "field '{}' expects {:?} but got {:?}",
                                name, expected.ty, value.ty
                            ),
                        );
                    }
                }
            }
        } else {
            self.diagnostics
                .error(span, format!("type '{}' is not a record type", type_name));
        }

        self.mk_expr(
            Type::Named(type_id, type_args),
            TirExprKind::RecordInit {
                type_id,
                fields: record_fields,
            },
        )
    }

    fn check_record_update_expr(
        &mut self,
        base: &Expr,
        fields: &[crate::ast::RecordFieldInit],
        span: crate::span::Span,
    ) -> TirExpr {
        let base_tir = self.check_expr(base);
        let (type_id, type_args) = match self.normalized_named_type(&base_tir.ty) {
            Some((type_id, type_args)) => (Some(type_id), type_args),
            None => (None, Vec::new()),
        };
        let updated = fields
            .iter()
            .map(|f| (f.name.clone(), self.check_expr(&f.value)))
            .collect();

        let Some(type_id) = type_id else {
            self.diagnostics
                .error(span, "record update base must be a named record type");
            return self.mk_expr(
                Type::Error,
                TirExprKind::RecordUpdate {
                    base: Box::new(base_tir),
                    type_id: None,
                    fields: updated,
                },
            );
        };

        if let Some(TypeInfo {
            kind: TypeKind::Record(expected_fields),
            ..
        }) = self.resolved.type_infos.get(type_id.0 as usize)
        {
            let subst = self.type_param_subst(type_id, &type_args);
            let expected_fields = self.instantiate_record_fields(expected_fields, &subst);
            for (name, value) in &updated {
                if let Some(expected) = expected_fields.iter().find(|f| f.name == *name) {
                    if !self.is_assignable(&expected.ty, &value.ty) {
                        self.diagnostics.error(
                            span,
                            format!(
                                "field '{}' expects {:?} but got {:?}",
                                name, expected.ty, value.ty
                            ),
                        );
                    }
                } else {
                    self.diagnostics
                        .error(span, format!("unknown field '{}' in record update", name));
                }
            }
        } else {
            self.diagnostics
                .error(span, "record update base must be a record type");
        }

        self.mk_expr(
            base_tir.ty.clone(),
            TirExprKind::RecordUpdate {
                base: Box::new(base_tir),
                type_id: Some(type_id),
                fields: updated,
            },
        )
    }

    fn check_constructor_expr(
        &mut self,
        name: &str,
        payload: &ConstructorPayload,
        span: crate::span::Span,
        expected: Option<&Type>,
    ) -> TirExpr {
        let Some(variant_id) = self.resolved.variant_names.get(name).copied() else {
            self.diagnostics
                .error(span, format!("unknown constructor '{}'", name));
            return self.mk_expr(Type::Error, TirExprKind::Unit);
        };
        let Some(type_id) = self.resolved.variant_to_type.get(&variant_id).copied() else {
            return self.mk_expr(Type::Error, TirExprKind::Unit);
        };

        let payload = match payload {
            ConstructorPayload::None => TirVariantPayload::None,
            ConstructorPayload::Positional(exprs) => {
                TirVariantPayload::Positional(exprs.iter().map(|e| self.check_expr(e)).collect())
            }
            ConstructorPayload::Record(fields) => TirVariantPayload::Record(
                fields
                    .iter()
                    .map(|f| (f.name.clone(), self.check_expr(&f.value)))
                    .collect(),
            ),
        };

        let mut type_args = self.default_type_args_for_type(type_id);
        if let Some((_, expected_payload_raw)) = self.variant_info(variant_id) {
            let mut subst: HashMap<TypeParamId, Type> = HashMap::new();
            infer_variant_payload_type_params(&expected_payload_raw, &payload, &mut subst);
            let context = format!("constructor '{}'", name);
            let expected_args = self.expected_named_type_args(expected, type_id);
            type_args = self.finalize_inferred_type_args(
                type_id,
                &mut subst,
                span,
                &context,
                expected_args,
            );
            let expected_payload = self.instantiate_variant_payload(&expected_payload_raw, &subst);
            self.check_constructor_payload(&expected_payload, &payload, span);
        }

        self.mk_expr(
            Type::Named(type_id, type_args),
            TirExprKind::VariantInit {
                variant_id,
                payload,
            },
        )
    }

    fn check_lambda_expr(&mut self, params: &[Param], ret_ty: &TypeExpr, body: &Expr) -> TirExpr {
        let ret_ty = self.lower_type_expr(ret_ty, false);
        self.push_scope();

        let mut tir_params = Vec::new();
        let mut arg_tys = Vec::new();
        for param in params {
            let ty = self.lower_type_expr(&param.ty, false);
            let local = self.alloc_local(param.name.clone(), ty.clone(), false, param.span);
            tir_params.push((local, ty.clone()));
            arg_tys.push(ty);
        }

        let body_expr = self.check_expr(body);
        if !self.is_assignable(&ret_ty, &body_expr.ty) {
            self.diagnostics.error(
                body.span,
                format!(
                    "lambda body has type {:?} but annotation expects {:?}",
                    body_expr.ty, ret_ty
                ),
            );
        }

        self.pop_scope();

        self.mk_expr(
            Type::Func(arg_tys, Box::new(ret_ty)),
            TirExprKind::Lambda {
                params: tir_params,
                body: Box::new(body_expr),
            },
        )
    }

    fn check_pattern(&mut self, pattern: &Pattern, scrutinee_ty: &Type) -> TirPattern {
        match &pattern.kind {
            PatternKind::Wildcard => TirPattern::Wildcard,
            PatternKind::Bind { name } => {
                let local =
                    self.alloc_local(name.clone(), scrutinee_ty.clone(), false, pattern.span);
                TirPattern::Bind(local)
            }
            PatternKind::Int { value } => {
                if !self.is_int_type(scrutinee_ty) {
                    self.diagnostics
                        .error(pattern.span, "integer pattern requires Int scrutinee");
                }
                TirPattern::Int(*value)
            }
            PatternKind::Bool { value } => {
                if !self.is_bool_type(scrutinee_ty) {
                    self.diagnostics
                        .error(pattern.span, "bool pattern requires Bool scrutinee");
                }
                TirPattern::Bool(*value)
            }
            PatternKind::String { value } => {
                if !self.is_string_type(scrutinee_ty) {
                    self.diagnostics
                        .error(pattern.span, "string pattern requires String scrutinee");
                }
                TirPattern::String(value.clone())
            }
            PatternKind::Constructor { name, args } => {
                let variant_id = self.resolved.variant_names.get(name).copied();
                let payload = if let Some(variant_id) = variant_id {
                    self.check_constructor_pattern_payload(
                        variant_id,
                        args,
                        &[],
                        scrutinee_ty,
                        pattern.span,
                    )
                } else {
                    self.diagnostics
                        .error(pattern.span, format!("unknown constructor '{}'", name));
                    TirPatternVariantPayload::Positional(
                        args.iter()
                            .map(|p| self.check_pattern(p, &Type::Error))
                            .collect(),
                    )
                };
                TirPattern::Variant {
                    variant_id,
                    payload,
                }
            }
            PatternKind::RecordConstructor { name, fields } => {
                let variant_id = self.resolved.variant_names.get(name).copied();
                let payload = if let Some(variant_id) = variant_id {
                    self.check_constructor_pattern_payload(
                        variant_id,
                        &[],
                        fields,
                        scrutinee_ty,
                        pattern.span,
                    )
                } else {
                    self.diagnostics
                        .error(pattern.span, format!("unknown constructor '{}'", name));
                    TirPatternVariantPayload::Record(
                        fields
                            .iter()
                            .map(|field| {
                                let value = if let Some(pat) = &field.pattern {
                                    self.check_pattern(pat, &Type::Error)
                                } else {
                                    let local = self.alloc_local(
                                        field.name.clone(),
                                        Type::Error,
                                        false,
                                        field.span,
                                    );
                                    TirPattern::Bind(local)
                                };
                                (field.name.clone(), value)
                            })
                            .collect(),
                    )
                };
                TirPattern::Variant {
                    variant_id,
                    payload,
                }
            }
        }
    }

    fn check_call_args(&mut self, span: crate::span::Span, params: &[Type], args: &[TirExpr]) {
        if params.len() != args.len() {
            self.diagnostics.error(
                span,
                format!(
                    "call argument count mismatch: expected {}, got {}",
                    params.len(),
                    args.len()
                ),
            );
            return;
        }
        for (idx, (expected, got)) in params.iter().zip(args).enumerate() {
            if !self.is_assignable(expected, &got.ty) {
                self.diagnostics.error(
                    span,
                    format!(
                        "argument {} expects {:?} but got {:?}",
                        idx + 1,
                        expected,
                        got.ty
                    ),
                );
            }
        }
    }

    fn rewrite_import_path(&self, path: &[String]) -> Vec<String> {
        let Some(first) = path.first() else {
            return Vec::new();
        };
        if let Some(base) = self.resolved.import_modules.get(first) {
            let mut out = base.clone();
            out.extend(path.iter().skip(1).cloned());
            out
        } else {
            path.to_vec()
        }
    }

    fn path_references_import_alias(&self, path: &[String]) -> bool {
        path.first()
            .is_some_and(|first| self.resolved.import_modules.contains_key(first))
    }

    fn is_imported_module_path(&self, segments: &[String]) -> bool {
        self.resolved
            .import_modules
            .values()
            .any(|path| segments.starts_with(path))
    }

    fn instantiate_func_signature(
        &mut self,
        func_id: FuncId,
        args: &[TirExpr],
        span: crate::span::Span,
    ) -> (Vec<Type>, Type) {
        let func_info = &self.resolved.func_infos[func_id.0 as usize];
        if func_info.type_params.is_empty() {
            return (func_info.params.clone(), func_info.ret.clone());
        }

        let mut subst: HashMap<TypeParamId, Type> = HashMap::new();
        for (expected, arg) in func_info.params.iter().zip(args) {
            let expected = self.normalize_type(expected);
            let actual = self.normalize_type(&arg.ty);
            infer_type_params(&expected, &actual, &mut subst);
        }
        for type_param in &func_info.type_params {
            if !subst.contains_key(type_param) {
                self.diagnostics.error(
                    span,
                    format!("could not infer generic type parameter {:?}", type_param),
                );
                subst.insert(*type_param, Type::Error);
            }
        }

        let params = func_info
            .params
            .iter()
            .map(|ty| substitute_type_params(ty, &subst))
            .collect();
        let ret = substitute_type_params(&func_info.ret, &subst);
        (params, ret)
    }

    fn unify_branch_type(
        &mut self,
        current: Option<Type>,
        next: Type,
        span: crate::span::Span,
        context: &str,
    ) -> Type {
        match current {
            None => next,
            Some(prev) => {
                if self.is_assignable(&prev, &next) {
                    prev
                } else if self.is_assignable(&next, &prev) {
                    next
                } else {
                    self.diagnostics.error(
                        span,
                        format!("incompatible {} types: {:?} vs {:?}", context, prev, next),
                    );
                    Type::Error
                }
            }
        }
    }

    fn sum_variants_for_type(&self, ty: &Type) -> Option<&[crate::types::VariantInfo]> {
        let (type_id, _) = self.normalized_named_type(ty)?;
        let info = self.resolved.type_infos.get(type_id.0 as usize)?;
        match &info.kind {
            TypeKind::Sum(variants) => Some(variants),
            _ => None,
        }
    }

    fn variant_info(&self, variant_id: VariantId) -> Option<(TypeId, VariantPayload)> {
        let type_id = self.resolved.variant_to_type.get(&variant_id).copied()?;
        let info = self.resolved.type_infos.get(type_id.0 as usize)?;
        match &info.kind {
            TypeKind::Sum(variants) => variants
                .iter()
                .find(|v| v.id == variant_id)
                .map(|v| (type_id, v.payload.clone())),
            _ => None,
        }
    }

    fn check_constructor_payload(
        &mut self,
        expected: &VariantPayload,
        got: &TirVariantPayload,
        span: crate::span::Span,
    ) {
        match (expected, got) {
            (VariantPayload::None, TirVariantPayload::None) => {}
            (VariantPayload::None, _) => {
                self.diagnostics
                    .error(span, "constructor does not accept a payload");
            }
            (VariantPayload::Positional(expected_tys), TirVariantPayload::Positional(values)) => {
                if expected_tys.len() != values.len() {
                    self.diagnostics.error(
                        span,
                        format!(
                            "constructor argument count mismatch: expected {}, got {}",
                            expected_tys.len(),
                            values.len()
                        ),
                    );
                }
                for (idx, (expected_ty, value)) in expected_tys.iter().zip(values).enumerate() {
                    if !self.is_assignable(expected_ty, &value.ty) {
                        self.diagnostics.error(
                            span,
                            format!(
                                "constructor argument {} expects {:?} but got {:?}",
                                idx + 1,
                                expected_ty,
                                value.ty
                            ),
                        );
                    }
                }
            }
            (VariantPayload::Record(expected_fields), TirVariantPayload::Record(values)) => {
                let provided: Vec<(String, crate::span::Span)> = values
                    .iter()
                    .map(|(name, _)| (name.clone(), span))
                    .collect();
                self.validate_record_field_set(span, expected_fields, &provided, "constructor");
                for (name, value) in values {
                    if let Some(expected) = expected_fields.iter().find(|f| f.name == *name) {
                        if !self.is_assignable(&expected.ty, &value.ty) {
                            self.diagnostics.error(
                                span,
                                format!(
                                    "field '{}' expects {:?} but got {:?}",
                                    name, expected.ty, value.ty
                                ),
                            );
                        }
                    }
                }
            }
            (VariantPayload::Positional(_), _) => {
                self.diagnostics
                    .error(span, "constructor requires positional payload");
            }
            (VariantPayload::Record(_), _) => {
                self.diagnostics
                    .error(span, "constructor requires record payload");
            }
        }
    }

    fn check_constructor_pattern_payload(
        &mut self,
        variant_id: VariantId,
        positional_args: &[Pattern],
        record_fields: &[RecordPatternField],
        scrutinee_ty: &Type,
        span: crate::span::Span,
    ) -> TirPatternVariantPayload {
        let Some((owner_type, expected_payload_raw)) = self.variant_info(variant_id) else {
            return TirPatternVariantPayload::None;
        };

        let mut subst: HashMap<TypeParamId, Type> = HashMap::new();
        if let Some((scrutinee_id, args)) = self.normalized_named_type(scrutinee_ty) {
            if scrutinee_id != owner_type {
                self.diagnostics
                    .error(span, "constructor pattern does not match scrutinee type");
            } else {
                subst = self.type_param_subst(owner_type, &args);
            }
        } else if !matches!(self.normalize_type(scrutinee_ty), Type::Error) {
            self.diagnostics
                .error(span, "constructor patterns require a sum-typed scrutinee");
        }

        let expected_payload = self.instantiate_variant_payload(&expected_payload_raw, &subst);
        match &expected_payload {
            VariantPayload::None => {
                if !positional_args.is_empty() || !record_fields.is_empty() {
                    self.diagnostics
                        .error(span, "constructor pattern takes no payload");
                }
                TirPatternVariantPayload::None
            }
            VariantPayload::Positional(expected_tys) => {
                if !record_fields.is_empty() {
                    self.diagnostics
                        .error(span, "constructor pattern requires positional arguments");
                }
                if expected_tys.len() != positional_args.len() {
                    self.diagnostics.error(
                        span,
                        format!(
                            "constructor pattern argument count mismatch: expected {}, got {}",
                            expected_tys.len(),
                            positional_args.len()
                        ),
                    );
                }
                let args = positional_args
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| {
                        let expected = expected_tys.get(idx).unwrap_or(&Type::Error);
                        self.check_pattern(arg, expected)
                    })
                    .collect();
                TirPatternVariantPayload::Positional(args)
            }
            VariantPayload::Record(expected_fields) => {
                if !positional_args.is_empty() {
                    self.diagnostics
                        .error(span, "constructor pattern requires record payload");
                }
                let provided: Vec<(String, crate::span::Span)> = record_fields
                    .iter()
                    .map(|field| (field.name.clone(), field.span))
                    .collect();
                self.validate_record_field_set(
                    span,
                    expected_fields,
                    &provided,
                    "constructor pattern",
                );
                let fields = record_fields
                    .iter()
                    .map(|field| {
                        let expected_ty = expected_fields
                            .iter()
                            .find(|f| f.name == field.name)
                            .map(|f| f.ty.clone())
                            .unwrap_or(Type::Error);
                        let value = if let Some(pat) = &field.pattern {
                            self.check_pattern(pat, &expected_ty)
                        } else {
                            let local = self.alloc_local(
                                field.name.clone(),
                                expected_ty,
                                false,
                                field.span,
                            );
                            TirPattern::Bind(local)
                        };
                        (field.name.clone(), value)
                    })
                    .collect();
                TirPatternVariantPayload::Record(fields)
            }
        }
    }

    fn validate_record_field_set(
        &mut self,
        span: crate::span::Span,
        expected_fields: &[crate::types::FieldInfo],
        provided_fields: &[(String, crate::span::Span)],
        context: &str,
    ) {
        let expected_names: HashSet<&str> =
            expected_fields.iter().map(|f| f.name.as_str()).collect();
        let mut seen = HashSet::new();
        for (name, field_span) in provided_fields {
            if !expected_names.contains(name.as_str()) {
                self.diagnostics.error(
                    *field_span,
                    format!("unknown field '{}' in {}", name, context),
                );
            }
            if !seen.insert(name.clone()) {
                self.diagnostics.error(
                    *field_span,
                    format!("duplicate field '{}' in {}", name, context),
                );
            }
        }

        for expected in expected_fields {
            if !seen.contains(&expected.name) {
                self.diagnostics.error(
                    span,
                    format!("missing field '{}' in {}", expected.name, context),
                );
            }
        }
    }

    fn default_type_args_for_type(&self, type_id: TypeId) -> Vec<Type> {
        self.resolved
            .type_infos
            .get(type_id.0 as usize)
            .map(|info| info.params.iter().map(|_| Type::Error).collect())
            .unwrap_or_default()
    }

    fn type_param_subst(&self, type_id: TypeId, type_args: &[Type]) -> HashMap<TypeParamId, Type> {
        let mut subst = HashMap::new();
        if let Some(info) = self.resolved.type_infos.get(type_id.0 as usize) {
            for (param, arg) in info.params.iter().zip(type_args) {
                subst.insert(*param, arg.clone());
            }
        }
        subst
    }

    fn finalize_inferred_type_args(
        &mut self,
        type_id: TypeId,
        subst: &mut HashMap<TypeParamId, Type>,
        span: crate::span::Span,
        context: &str,
        expected_args: Option<Vec<Type>>,
    ) -> Vec<Type> {
        let Some(info) = self.resolved.type_infos.get(type_id.0 as usize) else {
            return Vec::new();
        };
        let expected_args = expected_args.unwrap_or_default();

        let mut args = Vec::with_capacity(info.params.len());
        for (idx, type_param) in info.params.iter().enumerate() {
            if let Some(ty) = subst.get(type_param).cloned() {
                args.push(ty);
            } else if let Some(expected) = expected_args.get(idx).cloned() {
                args.push(expected.clone());
                subst.insert(*type_param, expected);
            } else {
                self.diagnostics.error(
                    span,
                    format!(
                        "could not infer generic type parameter {:?} for {}",
                        type_param, context
                    ),
                );
                args.push(Type::Error);
                subst.insert(*type_param, Type::Error);
            }
        }
        args
    }

    fn instantiate_record_fields(
        &self,
        fields: &[crate::types::FieldInfo],
        subst: &HashMap<TypeParamId, Type>,
    ) -> Vec<crate::types::FieldInfo> {
        fields
            .iter()
            .map(|field| crate::types::FieldInfo {
                name: field.name.clone(),
                ty: substitute_type_params(&field.ty, subst),
            })
            .collect()
    }

    fn instantiate_variant_payload(
        &self,
        payload: &VariantPayload,
        subst: &HashMap<TypeParamId, Type>,
    ) -> VariantPayload {
        match payload {
            VariantPayload::None => VariantPayload::None,
            VariantPayload::Positional(tys) => VariantPayload::Positional(
                tys.iter()
                    .map(|ty| substitute_type_params(ty, subst))
                    .collect(),
            ),
            VariantPayload::Record(fields) => {
                VariantPayload::Record(self.instantiate_record_fields(fields, subst))
            }
        }
    }

    fn normalize_type(&self, ty: &Type) -> Type {
        self.normalize_type_with_depth(ty, 0)
    }

    fn normalize_type_with_depth(&self, ty: &Type, depth: usize) -> Type {
        if depth > 64 {
            return Type::Error;
        }

        match ty {
            Type::Named(type_id, args) => {
                let args: Vec<Type> = args
                    .iter()
                    .map(|arg| self.normalize_type_with_depth(arg, depth + 1))
                    .collect();
                if let Some(info) = self.resolved.type_infos.get(type_id.0 as usize) {
                    if let TypeKind::Alias(alias_ty) = &info.kind {
                        let mut subst = HashMap::new();
                        for (param, arg) in info.params.iter().zip(&args) {
                            subst.insert(*param, arg.clone());
                        }
                        for param in info.params.iter().skip(args.len()) {
                            subst.insert(*param, Type::Error);
                        }
                        let expanded = substitute_type_params(alias_ty, &subst);
                        return self.normalize_type_with_depth(&expanded, depth + 1);
                    }
                }
                Type::Named(*type_id, args)
            }
            Type::Func(params, ret) => Type::Func(
                params
                    .iter()
                    .map(|p| self.normalize_type_with_depth(p, depth + 1))
                    .collect(),
                Box::new(self.normalize_type_with_depth(ret, depth + 1)),
            ),
            Type::ForeignNullable(inner) => {
                Type::ForeignNullable(Box::new(self.normalize_type_with_depth(inner, depth + 1)))
            }
            other => other.clone(),
        }
    }

    fn is_assignable(&self, expected: &Type, actual: &Type) -> bool {
        let expected = self.normalize_type(expected);
        let actual = self.normalize_type(actual);
        expected.is_assignable_from(&actual)
    }

    fn is_bool_type(&self, ty: &Type) -> bool {
        matches!(self.normalize_type(ty), Type::Bool | Type::Error)
    }

    fn is_int_type(&self, ty: &Type) -> bool {
        matches!(self.normalize_type(ty), Type::Int | Type::Error)
    }

    fn is_string_type(&self, ty: &Type) -> bool {
        matches!(self.normalize_type(ty), Type::String | Type::Error)
    }

    fn is_numeric_type(&self, ty: &Type) -> bool {
        is_numeric(&self.normalize_type(ty))
    }

    fn normalized_named_type(&self, ty: &Type) -> Option<(TypeId, Vec<Type>)> {
        match self.normalize_type(ty) {
            Type::Named(type_id, args) => Some((type_id, args)),
            _ => None,
        }
    }

    fn expected_named_type_args(
        &self,
        expected: Option<&Type>,
        type_id: TypeId,
    ) -> Option<Vec<Type>> {
        let expected = expected?;
        match self.normalize_type(expected) {
            Type::Named(expected_id, args) if expected_id == type_id => Some(args),
            _ => None,
        }
    }

    fn lower_type_expr(&mut self, expr: &TypeExpr, extern_ctx: bool) -> Type {
        match &expr.kind {
            TypeExprKind::Named { name, args } => {
                if let Some(param) = self.current_type_params.get(name).copied() {
                    return Type::TypeParam(param);
                }
                if let Some(ty) = builtin_type(name) {
                    return ty;
                }
                let Some(type_id) = self.resolved.type_names.get(name).copied() else {
                    self.diagnostics
                        .error(expr.span, format!("unknown type '{}'", name));
                    return Type::Error;
                };
                let args = args
                    .iter()
                    .map(|a| self.lower_type_expr(a, extern_ctx))
                    .collect();
                Type::Named(type_id, args)
            }
            TypeExprKind::Func { params, ret } => {
                let params = params
                    .iter()
                    .map(|p| self.lower_type_expr(p, extern_ctx))
                    .collect();
                let ret = self.lower_type_expr(ret, extern_ctx);
                Type::Func(params, Box::new(ret))
            }
            TypeExprKind::Nullable { inner } => {
                if !extern_ctx {
                    self.diagnostics
                        .error(expr.span, "nullable type only allowed in extern contexts");
                }
                Type::ForeignNullable(Box::new(self.lower_type_expr(inner, true)))
            }
            TypeExprKind::Nil => {
                if !extern_ctx {
                    self.diagnostics
                        .error(expr.span, "nil type only allowed in extern contexts");
                }
                Type::ForeignNil
            }
            TypeExprKind::Unit => Type::Unit,
        }
    }

    fn lower_func_kind(&self, kind: &FuncKind) -> TirFuncKind {
        match kind {
            FuncKind::Normal => TirFuncKind::Normal,
            FuncKind::Extern => TirFuncKind::Extern,
            FuncKind::Method { self_type } => TirFuncKind::Method {
                self_type: *self_type,
            },
        }
    }

    fn lookup_local(&self, name: &str) -> Option<&LocalBinding> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn alloc_local(
        &mut self,
        name: String,
        ty: Type,
        mutable: bool,
        span: crate::span::Span,
    ) -> LocalId {
        let local = LocalId(self.next_local_id);
        self.next_local_id += 1;

        let Some(scope) = self.scopes.last_mut() else {
            return local;
        };

        if scope
            .insert(
                name.clone(),
                LocalBinding {
                    id: local,
                    ty,
                    mutable,
                },
            )
            .is_some()
        {
            self.diagnostics
                .error(span, format!("duplicate local binding '{}'", name));
        }
        local
    }

    fn mk_expr(&self, ty: Type, kind: TirExprKind) -> TirExpr {
        TirExpr { ty, kind }
    }

    fn lookup_record_field(
        &self,
        type_id: TypeId,
        type_args: &[Type],
        field: &str,
    ) -> Option<Type> {
        let TypeInfo { kind, .. } = self.resolved.type_infos.get(type_id.0 as usize)?;
        match kind {
            TypeKind::Record(fields) => {
                let ty = fields
                    .iter()
                    .find(|f| f.name == field)
                    .map(|f| f.ty.clone())?;
                let subst = self.type_param_subst(type_id, type_args);
                Some(substitute_type_params(&ty, &subst))
            }
            _ => None,
        }
    }

    fn variant_payload(&self, variant_id: VariantId) -> Option<&VariantPayload> {
        let type_id = self.resolved.variant_to_type.get(&variant_id)?;
        let info = self.resolved.type_infos.get(type_id.0 as usize)?;
        match &info.kind {
            TypeKind::Sum(variants) => variants
                .iter()
                .find(|v| v.id == variant_id)
                .map(|v| &v.payload),
            _ => None,
        }
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::Float | Type::Error)
}

fn infer_variant_payload_type_params(
    expected: &VariantPayload,
    actual: &TirVariantPayload,
    subst: &mut HashMap<TypeParamId, Type>,
) {
    match (expected, actual) {
        (VariantPayload::None, TirVariantPayload::None) => {}
        (VariantPayload::Positional(expected_tys), TirVariantPayload::Positional(values)) => {
            for (expected_ty, value) in expected_tys.iter().zip(values) {
                infer_type_params(expected_ty, &value.ty, subst);
            }
        }
        (VariantPayload::Record(expected_fields), TirVariantPayload::Record(values)) => {
            infer_record_field_type_params(expected_fields, values, subst);
        }
        _ => {}
    }
}

fn infer_record_field_type_params(
    expected_fields: &[crate::types::FieldInfo],
    values: &[(String, TirExpr)],
    subst: &mut HashMap<TypeParamId, Type>,
) {
    for (name, value) in values {
        if let Some(expected) = expected_fields.iter().find(|f| f.name == *name) {
            infer_type_params(&expected.ty, &value.ty, subst);
        }
    }
}

fn infer_type_params(expected: &Type, actual: &Type, subst: &mut HashMap<TypeParamId, Type>) {
    match expected {
        Type::TypeParam(id) => {
            if let Some(existing) = subst.get(id) {
                if !existing.is_assignable_from(actual) && !actual.is_assignable_from(existing) {
                    subst.insert(*id, Type::Error);
                }
            } else {
                subst.insert(*id, actual.clone());
            }
        }
        Type::Named(expected_id, expected_args) => {
            if let Type::Named(actual_id, actual_args) = actual {
                if expected_id == actual_id && expected_args.len() == actual_args.len() {
                    for (exp_arg, act_arg) in expected_args.iter().zip(actual_args) {
                        infer_type_params(exp_arg, act_arg, subst);
                    }
                }
            }
        }
        Type::Func(expected_params, expected_ret) => {
            if let Type::Func(actual_params, actual_ret) = actual {
                if expected_params.len() == actual_params.len() {
                    for (exp_param, act_param) in expected_params.iter().zip(actual_params) {
                        infer_type_params(exp_param, act_param, subst);
                    }
                    infer_type_params(expected_ret, actual_ret, subst);
                }
            }
        }
        Type::ForeignNullable(expected_inner) => {
            if let Type::ForeignNullable(actual_inner) = actual {
                infer_type_params(expected_inner, actual_inner, subst);
            }
        }
        _ => {}
    }
}

fn substitute_type_params(ty: &Type, subst: &HashMap<TypeParamId, Type>) -> Type {
    match ty {
        Type::TypeParam(id) => subst.get(id).cloned().unwrap_or(Type::Error),
        Type::Named(type_id, args) => Type::Named(
            *type_id,
            args.iter()
                .map(|arg| substitute_type_params(arg, subst))
                .collect(),
        ),
        Type::Func(params, ret) => Type::Func(
            params
                .iter()
                .map(|param| substitute_type_params(param, subst))
                .collect(),
            Box::new(substitute_type_params(ret, subst)),
        ),
        Type::ForeignNullable(inner) => {
            Type::ForeignNullable(Box::new(substitute_type_params(inner, subst)))
        }
        other => other.clone(),
    }
}

fn builtin_type(name: &str) -> Option<Type> {
    Some(match name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "Unit" => Type::Unit,
        _ => return None,
    })
}
