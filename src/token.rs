use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    IntLit,
    FloatLit,
    StringLit,

    KwModule,
    KwImport,
    KwPub,
    KwExtern,
    KwType,
    KwFn,
    KwImpl,
    KwLet,
    KwVar,
    KwIf,
    KwThen,
    KwElseIf,
    KwElse,
    KwMatch,
    KwCase,
    KwWhile,
    KwDo,
    KwFor,
    KwIn,
    KwReturn,
    KwEnd,
    KwTrue,
    KwFalse,
    KwAnd,
    KwOr,
    KwNot,
    KwWith,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Arrow,
    FatArrow,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    DotDot,

    Newline,
    Eof,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: String) -> Self {
        Self { kind, span, lexeme }
    }
}

pub fn keyword_kind(s: &str) -> Option<TokenKind> {
    Some(match s {
        "module" => TokenKind::KwModule,
        "import" => TokenKind::KwImport,
        "pub" => TokenKind::KwPub,
        "extern" => TokenKind::KwExtern,
        "type" => TokenKind::KwType,
        "fn" => TokenKind::KwFn,
        "impl" => TokenKind::KwImpl,
        "let" => TokenKind::KwLet,
        "var" => TokenKind::KwVar,
        "if" => TokenKind::KwIf,
        "then" => TokenKind::KwThen,
        "elseif" => TokenKind::KwElseIf,
        "else" => TokenKind::KwElse,
        "match" => TokenKind::KwMatch,
        "case" => TokenKind::KwCase,
        "while" => TokenKind::KwWhile,
        "do" => TokenKind::KwDo,
        "for" => TokenKind::KwFor,
        "in" => TokenKind::KwIn,
        "return" => TokenKind::KwReturn,
        "end" => TokenKind::KwEnd,
        "true" => TokenKind::KwTrue,
        "false" => TokenKind::KwFalse,
        "and" => TokenKind::KwAnd,
        "or" => TokenKind::KwOr,
        "not" => TokenKind::KwNot,
        "with" => TokenKind::KwWith,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{Token, TokenKind, keyword_kind};
    use crate::span::Span;

    #[test]
    fn recognizes_keywords_and_non_keywords() {
        assert_eq!(keyword_kind("fn"), Some(TokenKind::KwFn));
        assert_eq!(keyword_kind("match"), Some(TokenKind::KwMatch));
        assert_eq!(keyword_kind("nope"), None);
    }

    #[test]
    fn token_constructor_sets_fields() {
        let span = Span::new(3, 1, 4);
        let token = Token::new(TokenKind::Ident, span, "name".to_string());
        assert_eq!(token.kind, TokenKind::Ident);
        assert_eq!(token.span, span);
        assert_eq!(token.lexeme, "name");
    }
}
