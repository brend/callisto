use std::path::{Path, PathBuf};

use crate::span::FileId;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: FileId,
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct SourceDb {
    files: Vec<SourceFile>,
}

impl SourceDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: PathBuf, text: String) -> FileId {
        let id = self.files.len() as FileId;
        self.files.push(SourceFile { id, path, text });
        id
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<FileId> {
        let path = path.as_ref().to_path_buf();
        let text = std::fs::read_to_string(&path)?;
        Ok(self.add_file(path, text))
    }

    pub fn get(&self, file_id: FileId) -> Option<&SourceFile> {
        self.files.get(file_id as usize)
    }

    pub fn line_col(&self, file_id: FileId, offset: u32) -> Option<(usize, usize)> {
        let file = self.get(file_id)?;
        let mut line = 1usize;
        let mut col = 1usize;
        for (idx, ch) in file.text.char_indices() {
            if idx as u32 >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        Some((line, col))
    }

    pub fn file_name(&self, file_id: FileId) -> &str {
        self.get(file_id)
            .and_then(|f| f.path.to_str())
            .unwrap_or("<unknown>")
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::SourceDb;

    #[test]
    fn add_file_and_line_col_work() {
        let mut db = SourceDb::new();
        let file_id = db.add_file(PathBuf::from("memory.luna"), "a\nbc\n".to_string());
        assert_eq!(db.file_name(file_id), "memory.luna");
        assert_eq!(db.line_col(file_id, 0), Some((1, 1)));
        assert_eq!(db.line_col(file_id, 2), Some((2, 1)));
        assert_eq!(db.line_col(file_id, 3), Some((2, 2)));
    }

    #[test]
    fn load_file_reads_contents() {
        let mut db = SourceDb::new();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("callisto_source_test_{}.luna", nonce));
        std::fs::write(&path, "fn main() -> Int do\n1\nend\n").expect("failed to write temp file");

        let file_id = db.load_file(&path).expect("failed to load temp file");
        let file = db.get(file_id).expect("loaded file missing");
        assert_eq!(file.text, "fn main() -> Int do\n1\nend\n");

        let _ = std::fs::remove_file(path);
    }
}
