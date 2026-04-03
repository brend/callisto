use crate::{
    ast::*,
    diagnostics::Diagnostics,
    span::Span,
    token::{Token, TokenKind},
};

pub fn parse(tokens: Vec<Token>) -> (Module, Diagnostics) {
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    (module, parser.diagnostics)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Diagnostics,
    next_expr_id: u32,
    next_pattern_id: u32,
    next_type_expr_id: u32,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
            next_expr_id: 0,
            next_pattern_id: 0,
            next_type_expr_id: 0,
        }
    }

    fn parse_module(&mut self) -> Module {
        let mut module_decl = None;
        let mut imports = Vec::new();
        let mut decls = Vec::new();

        self.skip_newlines();

        if self.at(TokenKind::KwModule) {
            module_decl = Some(self.parse_module_decl());
            self.skip_newlines();
        }

        while self.at(TokenKind::KwImport) {
            imports.push(self.parse_import_decl());
            self.skip_newlines();
        }

        while !self.at(TokenKind::Eof) {
            self.skip_newlines();
            if self.at(TokenKind::Eof) {
                break;
            }
            if let Some(decl) = self.parse_top_decl() {
                decls.push(decl);
            } else {
                self.recover_to_top_level();
            }
            self.skip_newlines();
        }

        Module {
            module_decl,
            imports,
            decls,
        }
    }

    fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.expect(TokenKind::KwModule, "expected 'module'").span;
        let path = self.parse_path();
        let end = self.prev_span();
        ModuleDecl {
            span: start.merge(end),
            path,
        }
    }

    fn parse_import_decl(&mut self) -> ImportDecl {
        let start = self.expect(TokenKind::KwImport, "expected 'import'").span;
        let path = self.parse_path();
        let items = if self.eat(TokenKind::Dot).is_some() && self.eat(TokenKind::LBrace).is_some() {
            let mut names = Vec::new();
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                let ident = self.expect_ident("expected imported item name");
                names.push(ident.lexeme);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected '}' after import items");
            Some(names)
        } else {
            None
        };
        let end = self.prev_span();
        ImportDecl {
            span: start.merge(end),
            path,
            items,
        }
    }

    fn parse_top_decl(&mut self) -> Option<TopDecl> {
        let vis = self.parse_visibility();

        if self.at(TokenKind::KwExtern) {
            self.bump();
            if self.at(TokenKind::KwType) {
                return Some(TopDecl::ExternType(self.parse_extern_type_decl(vis)));
            }
            if self.at(TokenKind::KwFn) {
                return Some(TopDecl::ExternFunc(self.parse_extern_func_decl(vis)));
            }
            if self.at(TokenKind::KwModule) {
                return Some(TopDecl::ExternModule(self.parse_extern_module_decl(vis)));
            }
            self.error_here("expected 'type', 'fn', or 'module' after 'extern'");
            return None;
        }

        if self.at(TokenKind::KwType) {
            return Some(TopDecl::Type(self.parse_type_decl(vis)));
        }
        if self.at(TokenKind::KwFn) {
            return Some(TopDecl::Func(self.parse_func_decl(vis)));
        }
        if self.at(TokenKind::KwImpl) {
            return Some(TopDecl::Impl(self.parse_impl_decl()));
        }

        self.error_here("expected top-level declaration");
        None
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.eat(TokenKind::KwPub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    fn parse_type_decl(&mut self, vis: Visibility) -> TypeDecl {
        let start = self.expect(TokenKind::KwType, "expected 'type'").span;
        let name = self.expect_ident("expected type name").lexeme;
        let type_params = self.parse_type_param_list();

        let body = if self.eat(TokenKind::Eq).is_some() {
            if self.at(TokenKind::Pipe) {
                TypeDeclBody::Sum(self.parse_sum_variants())
            } else if self.is_sum_variant_start() {
                TypeDeclBody::Sum(self.parse_sum_variants_no_leading_pipe())
            } else {
                TypeDeclBody::Alias(self.parse_type_expr())
            }
        } else if self.eat(TokenKind::LBrace).is_some() {
            let fields = self.parse_record_field_types();
            self.expect(TokenKind::RBrace, "expected '}' after record fields");
            TypeDeclBody::Record(fields)
        } else {
            self.error_here("expected '=' or '{' in type declaration");
            TypeDeclBody::Alias(self.mk_type_expr(Span::dummy(), TypeExprKind::Unit))
        };

        let end = self.prev_span();
        TypeDecl {
            span: start.merge(end),
            vis,
            name,
            type_params,
            body,
        }
    }

    fn parse_sum_variants(&mut self) -> Vec<SumVariantDecl> {
        let mut variants = Vec::new();
        while self.eat(TokenKind::Pipe).is_some() {
            variants.push(self.parse_sum_variant());
            self.skip_newlines();
        }
        variants
    }

    fn parse_sum_variants_no_leading_pipe(&mut self) -> Vec<SumVariantDecl> {
        let mut variants = Vec::new();
        variants.push(self.parse_sum_variant());
        while self.eat(TokenKind::Pipe).is_some() {
            variants.push(self.parse_sum_variant());
        }
        variants
    }

    fn parse_sum_variant(&mut self) -> SumVariantDecl {
        let name_tok = self.expect_ident("expected variant name");
        let span_start = name_tok.span;
        let name = name_tok.lexeme;

        let payload = if self.eat(TokenKind::LParen).is_some() {
            let mut tys = Vec::new();
            if !self.at(TokenKind::RParen) {
                loop {
                    tys.push(self.parse_type_expr());
                    if self.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "expected ')' after variant payload");
            SumVariantPayload::Positional(tys)
        } else if self.eat(TokenKind::LBrace).is_some() {
            let fields = self.parse_record_field_types();
            self.expect(TokenKind::RBrace, "expected '}' after record payload");
            SumVariantPayload::Record(fields)
        } else {
            SumVariantPayload::None
        };

        let end = self.prev_span();
        SumVariantDecl {
            span: span_start.merge(end),
            name,
            payload,
        }
    }

    fn parse_record_field_types(&mut self) -> Vec<RecordFieldType> {
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            self.skip_newlines();
            if self.at(TokenKind::RBrace) {
                break;
            }
            let name_tok = self.expect_ident("expected field name");
            self.expect(TokenKind::Colon, "expected ':' after field name");
            let ty = self.parse_type_expr();
            let span = name_tok.span.merge(ty.span);
            fields.push(RecordFieldType {
                span,
                name: name_tok.lexeme,
                ty,
            });
            if self.eat(TokenKind::Comma).is_none() {
                self.skip_newlines();
            }
        }
        fields
    }

    fn parse_extern_type_decl(&mut self, vis: Visibility) -> ExternTypeDecl {
        let start = self.expect(TokenKind::KwType, "expected 'type'").span;
        let name = self.expect_ident("expected extern type name").lexeme;
        let type_params = self.parse_type_param_list();
        let end = self.prev_span();
        ExternTypeDecl {
            span: start.merge(end),
            vis,
            name,
            type_params,
        }
    }

    fn parse_extern_func_decl(&mut self, vis: Visibility) -> ExternFuncDecl {
        let start = self.expect(TokenKind::KwFn, "expected 'fn'").span;
        let name = self.expect_ident("expected extern function name").lexeme;
        let params = self.parse_param_list();
        let ret_ty = if self.eat(TokenKind::Arrow).is_some() {
            self.parse_type_expr()
        } else {
            self.mk_type_expr(self.prev_span(), TypeExprKind::Unit)
        };
        let end = self.prev_span();
        ExternFuncDecl {
            span: start.merge(end),
            vis,
            name,
            params,
            ret_ty,
        }
    }

    fn parse_extern_module_decl(&mut self, vis: Visibility) -> ExternModuleDecl {
        let start = self.expect(TokenKind::KwModule, "expected 'module'").span;
        let path = self.parse_path();
        self.expect(TokenKind::KwDo, "expected 'do' in extern module");
        self.skip_newlines();
        let mut funcs = Vec::new();
        while !self.at(TokenKind::KwEnd) && !self.at(TokenKind::Eof) {
            let f_vis = self.parse_visibility();
            self.expect(
                TokenKind::KwExtern,
                "expected 'extern' for extern module functions",
            );
            funcs.push(self.parse_extern_func_decl(f_vis));
            self.skip_newlines();
        }
        self.expect(TokenKind::KwEnd, "expected 'end' after extern module");
        let end = self.prev_span();
        ExternModuleDecl {
            span: start.merge(end),
            vis,
            path,
            funcs,
        }
    }

    fn parse_impl_decl(&mut self) -> ImplDecl {
        let start = self.expect(TokenKind::KwImpl, "expected 'impl'").span;
        let target = self.expect_ident("expected impl target type").lexeme;
        self.expect(TokenKind::KwDo, "expected 'do' in impl");
        self.skip_newlines();
        let mut methods = Vec::new();
        while !self.at(TokenKind::KwEnd) && !self.at(TokenKind::Eof) {
            let vis = self.parse_visibility();
            methods.push(self.parse_func_decl(vis));
            self.skip_newlines();
        }
        self.expect(TokenKind::KwEnd, "expected 'end' after impl");
        let end = self.prev_span();
        ImplDecl {
            span: start.merge(end),
            target,
            methods,
        }
    }

    fn parse_func_decl(&mut self, vis: Visibility) -> FuncDecl {
        let start = self.expect(TokenKind::KwFn, "expected 'fn'").span;
        let name = self.expect_ident("expected function name").lexeme;
        let type_params = self.parse_type_param_list();
        let params = self.parse_param_list();
        let ret_ty = if self.eat(TokenKind::Arrow).is_some() {
            self.parse_type_expr()
        } else {
            self.mk_type_expr(self.prev_span(), TypeExprKind::Unit)
        };
        self.expect(TokenKind::KwDo, "expected 'do' before function body");
        let body = self.parse_block(&[TokenKind::KwEnd]);
        self.expect(TokenKind::KwEnd, "expected 'end' after function body");
        let end = self.prev_span();
        FuncDecl {
            span: start.merge(end),
            vis,
            name,
            type_params,
            params,
            ret_ty,
            body,
        }
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        self.expect(TokenKind::LParen, "expected '('");
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let name_tok = self.expect_ident("expected parameter name");
                self.expect(TokenKind::Colon, "expected ':' after parameter name");
                let ty = self.parse_type_expr();
                let span = name_tok.span.merge(ty.span);
                params.push(Param {
                    span,
                    name: name_tok.lexeme,
                    ty,
                });
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "expected ')' after parameter list");
        params
    }

    fn parse_type_param_list(&mut self) -> Vec<String> {
        if self.eat(TokenKind::LBracket).is_none() {
            return Vec::new();
        }
        let mut params = Vec::new();
        if !self.at(TokenKind::RBracket) {
            loop {
                params.push(self.expect_ident("expected type parameter").lexeme);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBracket, "expected ']' after type parameters");
        params
    }

    fn parse_block(&mut self, terminators: &[TokenKind]) -> Block {
        let start = self.current().span;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at_any(terminators) && !self.at(TokenKind::Eof) {
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.recover_to_stmt_boundary();
            }
            self.skip_newlines();
        }

        let tail = match stmts.last() {
            Some(Stmt::Expr(expr_stmt)) => Some(expr_stmt.expr.clone()),
            _ => None,
        };
        if tail.is_some() {
            stmts.pop();
        }

        let end = if self.pos == 0 {
            start
        } else {
            self.prev_span()
        };

        Block {
            span: start.merge(end),
            stmts,
            tail,
        }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        self.skip_newlines();
        if self.at_any(&[
            TokenKind::KwEnd,
            TokenKind::KwElse,
            TokenKind::KwElseIf,
            TokenKind::KwCase,
        ]) {
            return None;
        }

        if self.at(TokenKind::KwLet) {
            return Some(Stmt::Let(self.parse_let_stmt()));
        }
        if self.at(TokenKind::KwVar) {
            return Some(Stmt::Var(self.parse_var_stmt()));
        }
        if self.at(TokenKind::KwReturn) {
            return Some(Stmt::Return(self.parse_return_stmt()));
        }
        if self.at(TokenKind::KwWhile) {
            return Some(Stmt::While(self.parse_while_stmt()));
        }
        if self.at(TokenKind::KwFor) {
            return Some(Stmt::For(self.parse_for_stmt()));
        }

        if self.at(TokenKind::Ident) && self.peek_kind(1) == Some(TokenKind::Eq) {
            return Some(Stmt::Assign(self.parse_assign_stmt()));
        }

        let expr = self.parse_expr();
        let span = expr.span;
        Some(Stmt::Expr(ExprStmt { span, expr }))
    }

    fn parse_let_stmt(&mut self) -> LetStmt {
        let start = self.expect(TokenKind::KwLet, "expected 'let'").span;
        let name = self.expect_ident("expected binding name").lexeme;
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type_expr())
        } else {
            None
        };
        self.expect(TokenKind::Eq, "expected '=' in let binding");
        let value = self.parse_expr();
        let end = value.span;
        LetStmt {
            span: start.merge(end),
            name,
            ty,
            value,
        }
    }

    fn parse_var_stmt(&mut self) -> VarStmt {
        let start = self.expect(TokenKind::KwVar, "expected 'var'").span;
        let name = self.expect_ident("expected binding name").lexeme;
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type_expr())
        } else {
            None
        };
        self.expect(TokenKind::Eq, "expected '=' in var binding");
        let value = self.parse_expr();
        let end = value.span;
        VarStmt {
            span: start.merge(end),
            name,
            ty,
            value,
        }
    }

    fn parse_assign_stmt(&mut self) -> AssignStmt {
        let target_tok = self.expect_ident("expected assignment target");
        let start = target_tok.span;
        self.expect(TokenKind::Eq, "expected '=' in assignment");
        let value = self.parse_expr();
        AssignStmt {
            span: start.merge(value.span),
            target: target_tok.lexeme,
            value,
        }
    }

    fn parse_return_stmt(&mut self) -> ReturnStmt {
        let start = self.expect(TokenKind::KwReturn, "expected 'return'").span;
        let value = if self.at_any(&[
            TokenKind::Newline,
            TokenKind::KwEnd,
            TokenKind::KwElse,
            TokenKind::KwElseIf,
            TokenKind::KwCase,
            TokenKind::Eof,
        ]) {
            None
        } else {
            Some(self.parse_expr())
        };
        let end = value.as_ref().map(|v| v.span).unwrap_or(start);
        ReturnStmt {
            span: start.merge(end),
            value,
        }
    }

    fn parse_while_stmt(&mut self) -> WhileStmt {
        let start = self.expect(TokenKind::KwWhile, "expected 'while'").span;
        let cond = self.parse_expr();
        self.expect(TokenKind::KwDo, "expected 'do' in while statement");
        let body = self.parse_block(&[TokenKind::KwEnd]);
        self.expect(TokenKind::KwEnd, "expected 'end' after while body");
        let end = self.prev_span();
        WhileStmt {
            span: start.merge(end),
            cond,
            body,
        }
    }

    fn parse_for_stmt(&mut self) -> ForStmt {
        let start = self.expect(TokenKind::KwFor, "expected 'for'").span;
        let name = self.expect_ident("expected loop variable").lexeme;
        self.expect(TokenKind::KwIn, "expected 'in' in for loop");
        let start_expr = self.parse_expr();
        self.expect(TokenKind::DotDot, "expected '..' in for range");
        let end_expr = self.parse_expr();
        self.expect(TokenKind::KwDo, "expected 'do' in for loop");
        let body = self.parse_block(&[TokenKind::KwEnd]);
        self.expect(TokenKind::KwEnd, "expected 'end' after for loop");
        let end = self.prev_span();
        ForStmt {
            span: start.merge(end),
            name,
            start: start_expr,
            end: end_expr,
            body,
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix_expr();

        loop {
            if self.at(TokenKind::KwWith) {
                let (l_bp, r_bp) = (21, 22);
                if l_bp < min_bp {
                    break;
                }
                self.bump();
                self.expect(TokenKind::LBrace, "expected '{' after 'with'");
                let fields = self.parse_record_field_inits();
                self.expect(TokenKind::RBrace, "expected '}' after record update fields");
                let span = lhs.span.merge(self.prev_span());
                lhs = self.mk_expr(
                    span,
                    ExprKind::RecordUpdate {
                        base: Box::new(lhs),
                        fields,
                    },
                );
                let _ = r_bp;
                continue;
            }

            let Some((op, l_bp, r_bp)) = self.current_binary_op() else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr_bp(r_bp);
            let span = lhs.span.merge(rhs.span);
            lhs = self.mk_expr(
                span,
                ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
            );
        }

        lhs
    }

    fn parse_prefix_expr(&mut self) -> Expr {
        match self.current().kind {
            TokenKind::Minus => {
                let start = self.bump().span;
                let expr = self.parse_expr_bp(11);
                let span = start.merge(expr.span);
                self.mk_expr(
                    span,
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                )
            }
            TokenKind::KwNot => {
                let start = self.bump().span;
                let expr = self.parse_expr_bp(11);
                let span = start.merge(expr.span);
                self.mk_expr(
                    span,
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                )
            }
            _ => {
                let primary = self.parse_primary_expr();
                self.parse_postfix_expr(primary)
            }
        }
    }

    fn parse_primary_expr(&mut self) -> Expr {
        match self.current().kind {
            TokenKind::IntLit => {
                let tok = self.bump();
                let val = tok.lexeme.parse::<i64>().unwrap_or(0);
                self.mk_expr(tok.span, ExprKind::Int(val))
            }
            TokenKind::FloatLit => {
                let tok = self.bump();
                let val = tok.lexeme.parse::<f64>().unwrap_or(0.0);
                self.mk_expr(tok.span, ExprKind::Float(val))
            }
            TokenKind::StringLit => {
                let tok = self.bump();
                let raw = tok.lexeme;
                let s = raw
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(raw.as_str())
                    .to_string();
                self.mk_expr(tok.span, ExprKind::String(s))
            }
            TokenKind::KwTrue => {
                let tok = self.bump();
                self.mk_expr(tok.span, ExprKind::Bool(true))
            }
            TokenKind::KwFalse => {
                let tok = self.bump();
                self.mk_expr(tok.span, ExprKind::Bool(false))
            }
            TokenKind::Ident => {
                let tok = self.bump();
                self.mk_expr(tok.span, ExprKind::Var(tok.lexeme))
            }
            TokenKind::LParen => {
                let l = self.bump().span;
                if self.eat(TokenKind::RParen).is_some() {
                    return self.mk_expr(l, ExprKind::Unit);
                }
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "expected ')' ");
                let span = l.merge(self.prev_span());
                self.mk_expr(span, ExprKind::Paren(Box::new(inner)))
            }
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::KwFn => self.parse_lambda_expr(),
            _ => {
                self.error_here("expected expression");
                let tok = self.bump();
                self.mk_expr(tok.span, ExprKind::Unit)
            }
        }
    }

    fn parse_postfix_expr(&mut self, mut expr: Expr) -> Expr {
        loop {
            if self.eat(TokenKind::LParen).is_some() {
                let args = self.parse_expr_list(TokenKind::RParen);
                self.expect(TokenKind::RParen, "expected ')' after call arguments");
                let span = expr.span.merge(self.prev_span());
                expr = if let ExprKind::Var(name) = &expr.kind {
                    if is_constructor_name(name) {
                        self.mk_expr(
                            span,
                            ExprKind::Constructor {
                                name: name.clone(),
                                payload: ConstructorPayload::Positional(args),
                            },
                        )
                    } else {
                        self.mk_expr(
                            span,
                            ExprKind::Call {
                                callee: Box::new(expr),
                                args,
                            },
                        )
                    }
                } else {
                    self.mk_expr(
                        span,
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                    )
                };
                continue;
            }

            if self.eat(TokenKind::LBrace).is_some() {
                let fields = self.parse_record_field_inits();
                self.expect(TokenKind::RBrace, "expected '}' after fields");
                let span = expr.span.merge(self.prev_span());
                expr = match expr.kind {
                    ExprKind::Var(type_name) if is_constructor_name(&type_name) => {
                        self.mk_expr(span, ExprKind::RecordInit { type_name, fields })
                    }
                    _ => {
                        self.diagnostics
                            .error(span, "record init must target a named type");
                        self.mk_expr(span, ExprKind::Unit)
                    }
                };
                continue;
            }

            if self.eat(TokenKind::Dot).is_some() {
                let field_tok = self.expect_ident("expected field/method name after '.'");
                let name = field_tok.lexeme;
                let span = expr.span.merge(field_tok.span);
                if self.eat(TokenKind::LParen).is_some() {
                    let args = self.parse_expr_list(TokenKind::RParen);
                    self.expect(TokenKind::RParen, "expected ')' after method call");
                    expr = self.mk_expr(
                        span.merge(self.prev_span()),
                        ExprKind::MethodCall {
                            receiver: Box::new(expr),
                            method: name,
                            args,
                        },
                    );
                } else {
                    expr = self.mk_expr(
                        span,
                        ExprKind::Field {
                            receiver: Box::new(expr),
                            name,
                        },
                    );
                }
                continue;
            }

            break;
        }

        expr
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.expect(TokenKind::KwIf, "expected 'if'").span;
        let cond = self.parse_expr();
        self.expect(TokenKind::KwThen, "expected 'then' in if expression");
        let first_block =
            self.parse_block(&[TokenKind::KwElseIf, TokenKind::KwElse, TokenKind::KwEnd]);

        let mut branches = vec![(cond, first_block)];
        while self.at(TokenKind::KwElseIf) {
            self.bump();
            let cond = self.parse_expr();
            self.expect(TokenKind::KwThen, "expected 'then' in elseif expression");
            let block =
                self.parse_block(&[TokenKind::KwElseIf, TokenKind::KwElse, TokenKind::KwEnd]);
            branches.push((cond, block));
        }

        self.expect(TokenKind::KwElse, "expected 'else' in if expression");
        let else_block = self.parse_block(&[TokenKind::KwEnd]);
        self.expect(TokenKind::KwEnd, "expected 'end' after if expression");

        let span = start.merge(self.prev_span());
        self.mk_expr(
            span,
            ExprKind::If {
                branches,
                else_branch: Box::new(else_block),
            },
        )
    }

    fn parse_match_expr(&mut self) -> Expr {
        let start = self.expect(TokenKind::KwMatch, "expected 'match'").span;
        let scrutinee = self.parse_expr();
        if self.at(TokenKind::KwDo) {
            self.bump();
        }
        self.skip_newlines();

        let mut arms = Vec::new();
        while self.at(TokenKind::KwCase) {
            let case_span = self.bump().span;
            let pattern = self.parse_pattern();
            self.expect(TokenKind::FatArrow, "expected '=>' in match arm");
            let body = self.parse_block(&[TokenKind::KwCase, TokenKind::KwEnd]);
            let arm_span = case_span.merge(body.span);
            arms.push(MatchArm {
                span: arm_span,
                pattern,
                body,
            });
            self.skip_newlines();
        }

        self.expect(TokenKind::KwEnd, "expected 'end' after match expression");
        let span = start.merge(self.prev_span());
        self.mk_expr(
            span,
            ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
        )
    }

    fn parse_lambda_expr(&mut self) -> Expr {
        let start = self.expect(TokenKind::KwFn, "expected 'fn'").span;
        let params = self.parse_param_list();
        self.expect(TokenKind::Arrow, "expected '->' in lambda");
        let ret_ty = self.parse_type_expr();
        self.expect(TokenKind::FatArrow, "expected '=>' in lambda");
        let body = self.parse_expr();
        let span = start.merge(body.span);
        self.mk_expr(
            span,
            ExprKind::Lambda {
                params,
                ret_ty,
                body: Box::new(body),
            },
        )
    }

    fn parse_pattern(&mut self) -> Pattern {
        match self.current().kind {
            TokenKind::Ident => {
                let tok = self.bump();
                if tok.lexeme == "_" {
                    return self.mk_pattern(tok.span, PatternKind::Wildcard);
                }

                if is_constructor_name(&tok.lexeme) {
                    if self.eat(TokenKind::LParen).is_some() {
                        let mut args = Vec::new();
                        if !self.at(TokenKind::RParen) {
                            loop {
                                args.push(self.parse_pattern());
                                if self.eat(TokenKind::Comma).is_none() {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenKind::RParen, "expected ')' in constructor pattern");
                        let span = tok.span.merge(self.prev_span());
                        return self.mk_pattern(
                            span,
                            PatternKind::Constructor {
                                name: tok.lexeme,
                                args,
                            },
                        );
                    }
                    if self.eat(TokenKind::LBrace).is_some() {
                        let mut fields = Vec::new();
                        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                            let field_tok = self.expect_ident("expected record pattern field");
                            let pattern = if self.eat(TokenKind::Eq).is_some() {
                                Some(self.parse_pattern())
                            } else {
                                None
                            };
                            let span = pattern
                                .as_ref()
                                .map(|p| field_tok.span.merge(p.span))
                                .unwrap_or(field_tok.span);
                            fields.push(RecordPatternField {
                                span,
                                name: field_tok.lexeme,
                                pattern,
                            });
                            if self.eat(TokenKind::Comma).is_none() {
                                break;
                            }
                        }
                        self.expect(
                            TokenKind::RBrace,
                            "expected '}' in record constructor pattern",
                        );
                        let span = tok.span.merge(self.prev_span());
                        return self.mk_pattern(
                            span,
                            PatternKind::RecordConstructor {
                                name: tok.lexeme,
                                fields,
                            },
                        );
                    }
                }

                self.mk_pattern(tok.span, PatternKind::Bind { name: tok.lexeme })
            }
            TokenKind::IntLit => {
                let tok = self.bump();
                let value = tok.lexeme.parse().unwrap_or(0);
                self.mk_pattern(tok.span, PatternKind::Int { value })
            }
            TokenKind::StringLit => {
                let tok = self.bump();
                let value = tok.lexeme.trim_matches('"').to_string();
                self.mk_pattern(tok.span, PatternKind::String { value })
            }
            TokenKind::KwTrue => {
                let tok = self.bump();
                self.mk_pattern(tok.span, PatternKind::Bool { value: true })
            }
            TokenKind::KwFalse => {
                let tok = self.bump();
                self.mk_pattern(tok.span, PatternKind::Bool { value: false })
            }
            _ => {
                self.error_here("expected pattern");
                let tok = self.bump();
                self.mk_pattern(tok.span, PatternKind::Wildcard)
            }
        }
    }

    fn parse_type_expr(&mut self) -> TypeExpr {
        let lhs = self.parse_type_primary();
        if self.eat(TokenKind::Arrow).is_some() {
            let ret = self.parse_type_expr();
            let span = lhs.span.merge(ret.span);
            self.mk_type_expr(
                span,
                TypeExprKind::Func {
                    params: vec![lhs],
                    ret: Box::new(ret),
                },
            )
        } else {
            lhs
        }
    }

    fn parse_type_primary(&mut self) -> TypeExpr {
        match self.current().kind {
            TokenKind::Ident => {
                let tok = self.bump();
                let name = tok.lexeme;
                if name == "Nil" || name == "nil" {
                    return self.mk_type_expr(tok.span, TypeExprKind::Nil);
                }
                let mut args = Vec::new();
                if self.eat(TokenKind::LBracket).is_some() {
                    if !self.at(TokenKind::RBracket) {
                        loop {
                            args.push(self.parse_type_expr());
                            if self.eat(TokenKind::Comma).is_none() {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RBracket, "expected ']' after type arguments");
                }
                if self.eat(TokenKind::KwNot).is_some() {
                    let base = self.mk_type_expr(tok.span, TypeExprKind::Named { name, args });
                    let span = base.span.merge(self.prev_span());
                    return self.mk_type_expr(
                        span,
                        TypeExprKind::Nullable {
                            inner: Box::new(base),
                        },
                    );
                }
                self.mk_type_expr(tok.span, TypeExprKind::Named { name, args })
            }
            TokenKind::LParen => {
                let l = self.bump().span;
                if self.eat(TokenKind::RParen).is_some() {
                    return self.mk_type_expr(l, TypeExprKind::Unit);
                }
                let inner = self.parse_type_expr();
                self.expect(TokenKind::RParen, "expected ')' in type expression");
                let span = l.merge(self.prev_span());
                self.mk_type_expr(span, inner.kind)
            }
            _ => {
                self.error_here("expected type expression");
                let tok = self.bump();
                self.mk_type_expr(tok.span, TypeExprKind::Unit)
            }
        }
    }

    fn parse_path(&mut self) -> Vec<String> {
        let mut path = Vec::new();
        path.push(self.expect_ident("expected path segment").lexeme);
        while self.at(TokenKind::Dot) && self.peek_kind(1) != Some(TokenKind::LBrace) {
            self.bump();
            path.push(self.expect_ident("expected path segment after '.'").lexeme);
        }
        path
    }

    fn parse_expr_list(&mut self, terminator: TokenKind) -> Vec<Expr> {
        let mut args = Vec::new();
        if !self.at(terminator) {
            loop {
                args.push(self.parse_expr());
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        args
    }

    fn parse_record_field_inits(&mut self) -> Vec<RecordFieldInit> {
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            self.skip_newlines();
            if self.at(TokenKind::RBrace) {
                break;
            }
            let name_tok = self.expect_ident("expected field name");
            self.expect(TokenKind::Eq, "expected '=' in field initializer");
            let value = self.parse_expr();
            fields.push(RecordFieldInit {
                span: name_tok.span.merge(value.span),
                name: name_tok.lexeme,
                value,
            });
            if self.eat(TokenKind::Comma).is_none() {
                self.skip_newlines();
            }
        }
        fields
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        Some(match self.current().kind {
            TokenKind::KwOr => (BinaryOp::Or, 1, 2),
            TokenKind::KwAnd => (BinaryOp::And, 3, 4),
            TokenKind::EqEq => (BinaryOp::Eq, 5, 6),
            TokenKind::BangEq => (BinaryOp::NotEq, 5, 6),
            TokenKind::Lt => (BinaryOp::Lt, 7, 8),
            TokenKind::LtEq => (BinaryOp::LtEq, 7, 8),
            TokenKind::Gt => (BinaryOp::Gt, 7, 8),
            TokenKind::GtEq => (BinaryOp::GtEq, 7, 8),
            TokenKind::Plus => (BinaryOp::Add, 9, 10),
            TokenKind::Minus => (BinaryOp::Sub, 9, 10),
            TokenKind::Star => (BinaryOp::Mul, 11, 12),
            TokenKind::Slash => (BinaryOp::Div, 11, 12),
            TokenKind::Percent => (BinaryOp::Rem, 11, 12),
            _ => return None,
        })
    }

    fn is_sum_variant_start(&self) -> bool {
        self.at(TokenKind::Ident)
    }

    fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.iter().any(|k| self.at(*k))
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn peek_kind(&self, n: usize) -> Option<TokenKind> {
        self.tokens.get(self.pos + n).map(|t| t.kind)
    }

    fn current(&self) -> &Token {
        let idx = self.pos.min(self.tokens.len().saturating_sub(1));
        &self.tokens[idx]
    }

    fn bump(&mut self) -> Token {
        let tok = self.current().clone();
        self.pos = (self.pos + 1).min(self.tokens.len().saturating_sub(1));
        tok
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Token {
        if self.at(kind) {
            self.bump()
        } else {
            self.error_here(msg);
            self.bump()
        }
    }

    fn expect_ident(&mut self, msg: &str) -> Token {
        if self.at(TokenKind::Ident) {
            self.bump()
        } else {
            self.error_here(msg);
            self.bump()
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn skip_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn prev_span(&self) -> Span {
        if self.pos == 0 {
            self.current().span
        } else {
            self.tokens[self.pos - 1].span
        }
    }

    fn recover_to_top_level(&mut self) {
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::KwType)
            && !self.at(TokenKind::KwFn)
            && !self.at(TokenKind::KwImpl)
            && !self.at(TokenKind::KwExtern)
            && !self.at(TokenKind::KwPub)
        {
            self.bump();
        }
    }

    fn recover_to_stmt_boundary(&mut self) {
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::KwEnd)
            && !self.at(TokenKind::KwElse)
            && !self.at(TokenKind::KwElseIf)
            && !self.at(TokenKind::KwCase)
        {
            self.bump();
        }
    }

    fn error_here(&mut self, message: impl Into<String>) {
        self.diagnostics.error(self.current().span, message.into());
    }

    fn mk_expr(&mut self, span: Span, kind: ExprKind) -> Expr {
        let id = ExprId(self.next_expr_id);
        self.next_expr_id += 1;
        Expr { id, span, kind }
    }

    fn mk_pattern(&mut self, span: Span, kind: PatternKind) -> Pattern {
        let id = PatternId(self.next_pattern_id);
        self.next_pattern_id += 1;
        Pattern { id, span, kind }
    }

    fn mk_type_expr(&mut self, span: Span, kind: TypeExprKind) -> TypeExpr {
        let id = TypeExprId(self.next_type_expr_id);
        self.next_type_expr_id += 1;
        TypeExpr { id, span, kind }
    }
}

fn is_constructor_name(name: &str) -> bool {
    name.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}
