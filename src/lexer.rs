use crate::{
    diagnostics::Diagnostics,
    span::Span,
    token::{Token, TokenKind, keyword_kind},
};

pub fn lex(file_id: u32, text: &str) -> (Vec<Token>, Diagnostics) {
    let mut lexer = Lexer::new(file_id, text);
    lexer.lex_all();
    (lexer.tokens, lexer.diagnostics)
}

struct Lexer<'a> {
    file_id: u32,
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Diagnostics,
}

impl<'a> Lexer<'a> {
    fn new(file_id: u32, text: &'a str) -> Self {
        Self {
            file_id,
            text,
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Diagnostics::new(),
        }
    }

    fn lex_all(&mut self) {
        while let Some(ch) = self.peek_char() {
            match ch {
                ' ' | '\t' | '\r' => {
                    self.bump_char();
                }
                '\n' => {
                    let start = self.pos;
                    self.bump_char();
                    self.push_token(TokenKind::Newline, start, self.pos);
                }
                '/' if self.peek_next_char() == Some('/') => {
                    self.bump_char();
                    self.bump_char();
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.bump_char();
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident_or_keyword(),
                '0'..='9' => self.lex_number(),
                '"' => self.lex_string(),
                _ => self.lex_symbol(),
            }
        }
        self.tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.file_id, self.pos as u32, self.pos as u32),
            String::new(),
        ));
    }

    fn lex_ident_or_keyword(&mut self) {
        let start = self.pos;
        self.bump_char();
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.bump_char();
            } else {
                break;
            }
        }
        let lexeme = &self.text[start..self.pos];
        let kind = keyword_kind(lexeme).unwrap_or(TokenKind::Ident);
        self.tokens.push(Token::new(
            kind,
            Span::new(self.file_id, start as u32, self.pos as u32),
            lexeme.to_string(),
        ));
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        self.bump_char();
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.bump_char();
            } else {
                break;
            }
        }
        let mut kind = TokenKind::IntLit;
        if self.peek_char() == Some('.')
            && self.peek_next_char().is_some_and(|c| c.is_ascii_digit())
        {
            kind = TokenKind::FloatLit;
            self.bump_char();
            while let Some(ch) = self.peek_char() {
                if ch.is_ascii_digit() {
                    self.bump_char();
                } else {
                    break;
                }
            }
        }
        self.push_token(kind, start, self.pos);
    }

    fn lex_string(&mut self) {
        let start = self.pos;
        self.bump_char(); // opening quote

        let mut escaped = false;
        while let Some(ch) = self.peek_char() {
            self.bump_char();
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => {
                    self.push_token(TokenKind::StringLit, start, self.pos);
                    return;
                }
                _ => {}
            }
        }

        let span = Span::new(self.file_id, start as u32, self.pos as u32);
        self.diagnostics.error(span, "unterminated string literal");
        self.push_token(TokenKind::StringLit, start, self.pos);
    }

    fn lex_symbol(&mut self) {
        let start = self.pos;
        let Some(ch) = self.bump_char() else {
            return;
        };

        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '|' => TokenKind::Pipe,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,
            '.' => {
                if self.consume_if('.') {
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            '-' => {
                if self.consume_if('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '=' => {
                if self.consume_if('=') {
                    TokenKind::EqEq
                } else if self.consume_if('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.consume_if('=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Unknown
                }
            }
            '<' => {
                if self.consume_if('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.consume_if('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '/' => TokenKind::Slash,
            _ => TokenKind::Unknown,
        };

        if kind == TokenKind::Unknown {
            self.diagnostics.error(
                Span::new(self.file_id, start as u32, self.pos as u32),
                format!("unknown token '{}'", &self.text[start..self.pos]),
            );
        }

        self.push_token(kind, start, self.pos);
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token::new(
            kind,
            Span::new(self.file_id, start as u32, end as u32),
            self.text[start..end].to_string(),
        ));
    }

    fn peek_char(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut it = self.text[self.pos..].chars();
        it.next()?;
        it.next()
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.bump_char();
            true
        } else {
            false
        }
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::token::TokenKind;

    #[test]
    fn lexes_basic_tokens() {
        let (tokens, diags) = lex(0, "let x = 1 + 2\n");
        assert!(!diags.has_errors());
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::IntLit,
                TokenKind::Plus,
                TokenKind::IntLit,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_operators_and_comments() {
        let (tokens, diags) = lex(0, "a..b -> c => d != e // comment\n");
        assert!(!diags.has_errors());
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident,
                TokenKind::DotDot,
                TokenKind::Ident,
                TokenKind::Arrow,
                TokenKind::Ident,
                TokenKind::FatArrow,
                TokenKind::Ident,
                TokenKind::BangEq,
                TokenKind::Ident,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn reports_lex_errors_for_unterminated_string_and_unknown_token() {
        let (_, diags) = lex(0, "\"unterminated");
        assert!(
            diags
                .items
                .iter()
                .any(|d| d.message.contains("unterminated string literal"))
        );

        let (_, diags) = lex(0, "@");
        assert!(diags.has_errors());
        assert!(
            diags
                .items
                .iter()
                .any(|d| d.message.contains("unknown token '@'"))
        );
    }
}
