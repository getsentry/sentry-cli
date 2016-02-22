use std::path::{Path, PathBuf};
use std::io;
use std::fs;
use std::env;
use uuid::Uuid;

pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub fn new() -> io::Result<TempFile> {
        let mut path = env::temp_dir();
        path.push(Uuid::new_v4().to_hyphenated_string());
        Ok(TempFile {
            path: path
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
