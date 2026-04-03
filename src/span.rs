#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file_id: FileId,
    pub start: u32,
    pub end: u32,
}

pub type FileId = u32;

impl Span {
    pub const fn new(file_id: FileId, start: u32, end: u32) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    pub const fn dummy() -> Self {
        Self {
            file_id: 0,
            start: 0,
            end: 0,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        if self.file_id != other.file_id {
            return self;
        }
        Span {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    #[test]
    fn merge_combines_ranges_in_same_file() {
        let a = Span::new(1, 5, 10);
        let b = Span::new(1, 2, 7);
        let merged = a.merge(b);
        assert_eq!(merged, Span::new(1, 2, 10));
    }

    #[test]
    fn merge_keeps_left_span_for_different_files() {
        let a = Span::new(1, 5, 10);
        let b = Span::new(2, 0, 3);
        assert_eq!(a.merge(b), a);
    }
}
