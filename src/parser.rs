use crate::{
    ast::*,
    diagnostics::Diagnostics,
    span::Span,
    token::{Token, TokenKind},
};

const DIAG_PARSE_OLD_BLOCK_DELIMITER: &str = "CAL-PAR-001";
const DIAG_PARSE_ELSEIF_REMOVED: &str = "CAL-PAR-002";

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
        if self.at(TokenKind::KwNewtype) {
            return Some(TopDecl::Type(self.parse_newtype_decl(vis)));
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
            if self.peek_non_newline_kind() == Some(TokenKind::Pipe) {
                self.skip_newlines();
                TypeDeclBody::Sum(self.parse_sum_variants())
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

    fn parse_newtype_decl(&mut self, vis: Visibility) -> TypeDecl {
        let start = self.expect(TokenKind::KwNewtype, "expected 'newtype'").span;
        let name = self.expect_ident("expected newtype name").lexeme;
        let type_params = self.parse_type_param_list();
        self.expect(TokenKind::Eq, "expected '=' in newtype declaration");
        let inner = self.parse_type_expr();
        let end = self.prev_span();
        TypeDecl {
            span: start.merge(end),
            vis,
            name,
            type_params,
            body: TypeDeclBody::Newtype(inner),
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

    fn parse_sum_variant(&mut self) -> SumVariantDecl {
        let name_tok = self.expect_ident("expected variant name");
        let span_start = name_tok.span;
        let name = name_tok.lexeme;

        let payload = if self.eat(TokenKind::LParen).is_some() {
            let mut tys = Vec::new();
            self.skip_newlines();
            if !self.at(TokenKind::RParen) {
                loop {
                    self.skip_newlines();
                    if self.at(TokenKind::RParen) {
                        break;
                    }
                    tys.push(self.parse_type_expr());
                    self.skip_newlines();
                    if self.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                    if self.at(TokenKind::RParen) {
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
        let braced = self.expect_block_start("use `{ ... }` after an extern module path");
        self.skip_newlines();
        let mut funcs = Vec::new();
        let terminator = if braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        };
        while !self.at(terminator) && !self.at(TokenKind::Eof) {
            let f_vis = self.parse_visibility();
            self.expect(
                TokenKind::KwExtern,
                "expected 'extern' for extern module functions",
            );
            funcs.push(self.parse_extern_func_decl(f_vis));
            self.skip_newlines();
        }
        self.expect_block_end(braced, "expected '}' after extern module");
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
        let braced = self.expect_block_start("use `{ ... }` after an impl target");
        self.skip_newlines();
        let mut methods = Vec::new();
        let terminator = if braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        };
        while !self.at(terminator) && !self.at(TokenKind::Eof) {
            let vis = self.parse_visibility();
            methods.push(self.parse_func_decl(vis));
            self.skip_newlines();
        }
        self.expect_block_end(braced, "expected '}' after impl");
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
        let braced = self.expect_block_start("use `{ ... }` after a function signature");
        let terminator = if braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        };
        let body = self.parse_block(&[terminator]);
        self.expect_block_end(braced, "expected '}' after function body");
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
        self.skip_newlines();
        if !self.at(TokenKind::RParen) {
            loop {
                self.skip_newlines();
                if self.at(TokenKind::RParen) {
                    break;
                }
                let name_tok = self.expect_ident("expected parameter name");
                self.expect(TokenKind::Colon, "expected ':' after parameter name");
                let ty = self.parse_type_expr();
                let span = name_tok.span.merge(ty.span);
                params.push(Param {
                    span,
                    name: name_tok.lexeme,
                    ty,
                });
                self.skip_newlines();
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                self.skip_newlines();
                if self.at(TokenKind::RParen) {
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
        self.skip_newlines();
        if !self.at(TokenKind::RBracket) {
            loop {
                self.skip_newlines();
                if self.at(TokenKind::RBracket) {
                    break;
                }
                params.push(self.expect_ident("expected type parameter").lexeme);
                self.skip_newlines();
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                self.skip_newlines();
                if self.at(TokenKind::RBracket) {
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
            TokenKind::RBrace,
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
            TokenKind::RBrace,
            TokenKind::Comma,
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
        let braced = self.expect_block_start("use `{ ... }` after a while condition");
        let terminator = if braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        };
        let body = self.parse_block(&[terminator]);
        self.expect_block_end(braced, "expected '}' after while body");
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
        let braced = self.expect_block_start("use `{ ... }` after a for range");
        let terminator = if braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        };
        let body = self.parse_block(&[terminator]);
        self.expect_block_end(braced, "expected '}' after for loop");
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
                self.parse_string_literal_expr(tok)
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
            TokenKind::LBracket => {
                let l = self.bump().span;
                let items = self.parse_expr_list(TokenKind::RBracket);
                self.expect(TokenKind::RBracket, "expected ']' after list literal");
                let span = l.merge(self.prev_span());
                self.mk_expr(span, ExprKind::ListLiteral(items))
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

    fn parse_string_literal_expr(&mut self, tok: Token) -> Expr {
        let raw = tok.lexeme;
        let content = raw
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(raw.as_str());
        let content_start = tok.span.start + 1;
        let bytes = content.as_bytes();

        let mut parts = Vec::new();
        let mut saw_interpolation = false;
        let mut literal_start = 0usize;
        let mut i = 0usize;

        while i + 1 < bytes.len() {
            if bytes[i] == b'$' && bytes[i + 1] == b'{' {
                let slash_count = Self::count_preceding_backslashes(bytes, i);
                if slash_count % 2 == 1 {
                    i += 2;
                    continue;
                }

                let Some(end_idx) = Self::find_interpolation_end(content, i + 2) else {
                    let span = Span::new(tok.span.file_id, content_start + i as u32, tok.span.end);
                    self.diagnostics
                        .error(span, "unterminated string interpolation");
                    let literal = Self::unescape_interpolation_markers(content);
                    return self.mk_expr(tok.span, ExprKind::String(literal));
                };

                let literal = Self::unescape_interpolation_markers(&content[literal_start..i]);
                if !literal.is_empty() {
                    parts.push(StringPart::Text(literal));
                }

                let expr_source = &content[i + 2..end_idx];
                let expr_span_start = content_start + (i + 2) as u32;
                let expr = self.parse_string_interpolation_expr(
                    expr_source,
                    tok.span.file_id,
                    expr_span_start,
                    end_idx.saturating_sub(i + 2) as u32,
                );
                parts.push(StringPart::Expr(expr));
                saw_interpolation = true;

                i = end_idx + 1;
                literal_start = i;
                continue;
            }
            i += 1;
        }

        if !saw_interpolation {
            let s = Self::unescape_interpolation_markers(content);
            return self.mk_expr(tok.span, ExprKind::String(s));
        }

        let tail = Self::unescape_interpolation_markers(&content[literal_start..]);
        if !tail.is_empty() {
            parts.push(StringPart::Text(tail));
        }

        self.mk_expr(tok.span, ExprKind::StringInterp(parts))
    }

    fn parse_string_interpolation_expr(
        &mut self,
        source: &str,
        file_id: u32,
        span_start: u32,
        fallback_len: u32,
    ) -> Expr {
        let fallback_span = Span::new(file_id, span_start, span_start + fallback_len);
        if source.trim().is_empty() {
            self.diagnostics
                .error(fallback_span, "empty string interpolation expression");
            return self.mk_expr(fallback_span, ExprKind::String(String::new()));
        }

        let (mut tokens, mut lex_diags) = crate::lexer::lex(file_id, source);
        Self::offset_diagnostics(&mut lex_diags, span_start);
        self.diagnostics.extend(lex_diags);
        for tok in &mut tokens {
            tok.span = Self::offset_span(tok.span, span_start);
        }

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr();
        parser.skip_newlines();
        if !parser.at(TokenKind::Eof) {
            parser.error_here("unexpected tokens in string interpolation expression");
        }
        self.diagnostics.extend(parser.diagnostics);
        expr
    }

    fn offset_diagnostics(diags: &mut Diagnostics, offset: u32) {
        for diag in &mut diags.items {
            diag.primary_span = Self::offset_span(diag.primary_span, offset);
            for (span, _) in &mut diag.notes {
                *span = Self::offset_span(*span, offset);
            }
        }
    }

    fn offset_span(span: Span, offset: u32) -> Span {
        Span::new(
            span.file_id,
            span.start.saturating_add(offset),
            span.end.saturating_add(offset),
        )
    }

    fn count_preceding_backslashes(bytes: &[u8], idx: usize) -> usize {
        let mut count = 0usize;
        let mut cursor = idx;
        while cursor > 0 && bytes[cursor - 1] == b'\\' {
            count += 1;
            cursor -= 1;
        }
        count
    }

    fn unescape_interpolation_markers(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;
        let mut i = 0usize;

        while i + 1 < bytes.len() {
            if bytes[i] == b'$' && bytes[i + 1] == b'{' {
                let slash_count = Self::count_preceding_backslashes(bytes, i);
                if slash_count % 2 == 1 {
                    out.push_str(&text[cursor..i - 1]);
                    out.push_str("${");
                    i += 2;
                    cursor = i;
                    continue;
                }
            }
            i += 1;
        }

        out.push_str(&text[cursor..]);
        out
    }

    fn find_interpolation_end(text: &str, expr_start: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut i = expr_start;
        let mut depth = 1usize;
        let mut in_string = false;
        let mut escaped = false;

        while i < bytes.len() {
            let ch = bytes[i];
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == b'\\' {
                    escaped = true;
                } else if ch == b'"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            match ch {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }

            i += 1;
        }

        None
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

            if self.at(TokenKind::LBrace)
                && matches!(&expr.kind, ExprKind::Var(name) if is_constructor_name(name))
            {
                self.bump();
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

            if self.eat(TokenKind::LBracket).is_some() {
                let index = self.parse_expr();
                self.expect(TokenKind::RBracket, "expected ']' after index expression");
                let span = expr.span.merge(self.prev_span());
                expr = self.mk_expr(
                    span,
                    ExprKind::Index {
                        collection: Box::new(expr),
                        index: Box::new(index),
                    },
                );
                continue;
            }

            break;
        }

        expr
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.expect(TokenKind::KwIf, "expected 'if'").span;
        let cond = self.parse_expr();
        let first_braced = self.expect_block_start("use `{ ... }` after an if condition");
        let first_terminators = if first_braced {
            vec![TokenKind::RBrace]
        } else {
            vec![TokenKind::KwElseIf, TokenKind::KwElse, TokenKind::KwEnd]
        };
        let first_block = self.parse_block(&first_terminators);
        if first_braced {
            self.expect(TokenKind::RBrace, "expected '}' after if branch");
        }

        let mut branches = vec![(cond, first_block)];
        while self.at(TokenKind::KwElse) && self.peek_kind(1) == Some(TokenKind::KwIf) {
            self.bump();
            self.bump();
            let cond = self.parse_expr();
            self.expect(TokenKind::LBrace, "expected '{' after else if condition");
            let block = self.parse_block(&[TokenKind::RBrace]);
            self.expect(TokenKind::RBrace, "expected '}' after else if branch");
            branches.push((cond, block));
        }
        while self.at(TokenKind::KwElseIf) {
            self.error_code_here(
                DIAG_PARSE_ELSEIF_REMOVED,
                "use `else if cond { ... }` instead of `elseif cond then ... end`",
            );
            self.bump();
            let cond = self.parse_expr();
            let braced = self.expect_block_start("use `else if cond { ... }`");
            let block = self.parse_block(&[if braced {
                TokenKind::RBrace
            } else {
                TokenKind::KwEnd
            }]);
            self.expect_block_end(braced, "expected '}' after else if branch");
            branches.push((cond, block));
        }

        self.expect(TokenKind::KwElse, "expected 'else' in if expression");
        let else_braced = self.expect_block_start("use `{ ... }` after else");
        let else_block = self.parse_block(&[if else_braced {
            TokenKind::RBrace
        } else {
            TokenKind::KwEnd
        }]);
        self.expect_block_end(else_braced, "expected '}' after else branch");

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
        let braced = self.expect_block_start("use `{ ... }` after a match scrutinee");
        self.skip_newlines();

        let mut arms = Vec::new();
        while self.at(TokenKind::KwCase) {
            let case_span = self.bump().span;
            let pattern = self.parse_pattern();
            self.expect(TokenKind::FatArrow, "expected '=>' in match arm");
            let body_terminator = if braced {
                TokenKind::RBrace
            } else {
                TokenKind::KwEnd
            };
            let body = self.parse_block(&[TokenKind::KwCase, body_terminator, TokenKind::Comma]);
            let arm_span = case_span.merge(body.span);
            arms.push(MatchArm {
                span: arm_span,
                pattern,
                body,
            });
            let _ = self.eat(TokenKind::Comma);
            self.skip_newlines();
        }

        self.expect_block_end(braced, "expected '}' after match expression");
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
                        self.skip_newlines();
                        if !self.at(TokenKind::RParen) {
                            loop {
                                self.skip_newlines();
                                if self.at(TokenKind::RParen) {
                                    break;
                                }
                                args.push(self.parse_pattern());
                                self.skip_newlines();
                                if self.eat(TokenKind::Comma).is_none() {
                                    break;
                                }
                                self.skip_newlines();
                                if self.at(TokenKind::RParen) {
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
                            self.skip_newlines();
                            if self.at(TokenKind::RBrace) {
                                break;
                            }
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

                    return self.mk_pattern(
                        tok.span,
                        PatternKind::Constructor {
                            name: tok.lexeme,
                            args: Vec::new(),
                        },
                    );
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
        self.skip_newlines();
        if !self.at(terminator) {
            loop {
                self.skip_newlines();
                if self.at(terminator) {
                    break;
                }
                args.push(self.parse_expr());
                self.skip_newlines();
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                self.skip_newlines();
                if self.at(terminator) {
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
            let value = if self.eat(TokenKind::Eq).is_some() {
                self.parse_expr()
            } else {
                // Record field punning: `Point { x }` expands to `Point { x = x }`.
                self.mk_expr(name_tok.span, ExprKind::Var(name_tok.lexeme.clone()))
            };
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

    fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.iter().any(|k| self.at(*k))
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn peek_kind(&self, n: usize) -> Option<TokenKind> {
        self.tokens.get(self.pos + n).map(|t| t.kind)
    }

    fn peek_non_newline_kind(&self) -> Option<TokenKind> {
        let mut idx = self.pos;
        while let Some(tok) = self.tokens.get(idx) {
            if tok.kind != TokenKind::Newline {
                return Some(tok.kind);
            }
            idx += 1;
        }
        None
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

    fn expect_block_start(&mut self, replacement: &str) -> bool {
        if self.eat(TokenKind::LBrace).is_some() {
            return true;
        }
        if self.at(TokenKind::KwDo) || self.at(TokenKind::KwThen) {
            let found = self.current().lexeme.clone();
            self.error_code_here(
                DIAG_PARSE_OLD_BLOCK_DELIMITER,
                format!("old block delimiter `{found}` is no longer supported; {replacement}"),
            );
            self.bump();
            return false;
        }
        if self.at(TokenKind::KwEnd) {
            self.error_code_here(
                DIAG_PARSE_OLD_BLOCK_DELIMITER,
                "old block delimiter `end` is no longer supported; use `}`",
            );
        } else {
            self.error_here(format!("expected '{{'; {replacement}"));
        }
        false
    }

    fn expect_block_end(&mut self, braced: bool, msg: &str) {
        if braced {
            self.expect(TokenKind::RBrace, msg);
        } else if self.at(TokenKind::KwEnd) {
            self.error_code_here(
                DIAG_PARSE_OLD_BLOCK_DELIMITER,
                "old block delimiter `end` is no longer supported; use `}`",
            );
            self.bump();
        } else {
            self.expect(TokenKind::RBrace, msg);
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
            && !self.at(TokenKind::KwNewtype)
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
            && !self.at(TokenKind::RBrace)
            && !self.at(TokenKind::Comma)
            && !self.at(TokenKind::KwEnd)
            && !self.at(TokenKind::KwElse)
            && !self.at(TokenKind::KwElseIf)
            && !self.at(TokenKind::KwIf)
            && !self.at(TokenKind::KwCase)
        {
            self.bump();
        }
    }

    fn error_here(&mut self, message: impl Into<String>) {
        self.diagnostics.error(self.current().span, message.into());
    }

    fn error_code_here(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics
            .error_code(self.current().span, code, message.into());
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
