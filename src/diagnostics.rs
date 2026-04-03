use std::fmt::Write;

use crate::{source::SourceDb, span::Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub primary_span: Span,
    pub notes: Vec<(Span, String)>,
}

#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(&mut self, span: Span, message: impl Into<String>) {
        self.push(Diagnostic {
            level: DiagnosticLevel::Error,
            message: message.into(),
            primary_span: span,
            notes: Vec::new(),
        });
    }

    pub fn warning(&mut self, span: Span, message: impl Into<String>) {
        self.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            primary_span: span,
            notes: Vec::new(),
        });
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|d| matches!(d.level, DiagnosticLevel::Error))
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn render(&self, sources: &SourceDb) -> String {
        let mut out = String::new();
        for diag in &self.items {
            let (line, col) = sources
                .line_col(diag.primary_span.file_id, diag.primary_span.start)
                .unwrap_or((0, 0));
            let level = match diag.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
            };
            let _ = writeln!(
                out,
                "{}:{}:{}: {}: {}",
                sources.file_name(diag.primary_span.file_id),
                line,
                col,
                level,
                diag.message
            );
            for (note_span, note) in &diag.notes {
                let (n_line, n_col) = sources
                    .line_col(note_span.file_id, note_span.start)
                    .unwrap_or((0, 0));
                let _ = writeln!(
                    out,
                    "  note at {}:{}:{}: {}",
                    sources.file_name(note_span.file_id),
                    n_line,
                    n_col,
                    note
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Diagnostic, DiagnosticLevel, Diagnostics};
    use crate::{source::SourceDb, span::Span};

    #[test]
    fn tracks_errors_and_warnings() {
        let span = Span::new(0, 0, 1);
        let mut diagnostics = Diagnostics::new();
        diagnostics.warning(span, "warn");
        assert!(!diagnostics.has_errors());
        diagnostics.error(span, "err");
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn render_includes_primary_and_notes() {
        let mut db = SourceDb::new();
        let file_id = db.add_file(PathBuf::from("sample.luna"), "ab\ncd\n".to_string());
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            message: "bad thing".to_string(),
            primary_span: Span::new(file_id, 3, 4),
            notes: vec![(Span::new(file_id, 0, 1), "more detail".to_string())],
        });

        let rendered = diagnostics.render(&db);
        assert!(rendered.contains("sample.luna:2:1: error: bad thing"));
        assert!(rendered.contains("note at sample.luna:1:1: more detail"));
    }

    #[test]
    fn extend_appends_items() {
        let span = Span::new(0, 0, 1);
        let mut a = Diagnostics::new();
        let mut b = Diagnostics::new();
        a.error(span, "first");
        b.warning(span, "second");
        a.extend(b);
        assert_eq!(a.items.len(), 2);
    }
}
