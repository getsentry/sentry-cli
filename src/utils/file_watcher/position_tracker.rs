#![allow(clippy::allow_attributes)]

use anyhow::{Context as _, Result};
use log::debug;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Seek as _, SeekFrom};
use std::path::Path;

/// Tracks file position for tail-like behavior
/// Handles file rotation and growth scenarios
#[derive(Debug)]
pub struct PositionTracker {
    file_path: std::path::PathBuf,
    current_position: u64,
    current_size: u64,
    inode: Option<u64>,
}

impl PositionTracker {
    /// Create a new position tracker starting from the end of the file
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        let metadata = std::fs::metadata(&file_path)
            .with_context(|| format!("Failed to get metadata for {}", file_path.display()))?;

        let current_size = metadata.len();
        let inode = get_inode(&metadata);

        debug!(
            "Initialized position tracker for {} at position {} (size: {})",
            file_path.display(),
            current_size,
            current_size
        );

        Ok(PositionTracker {
            file_path,
            current_position: current_size,
            current_size,
            inode,
        })
    }

    /// Check if the file has new data since the last read
    /// Returns the number of new bytes available
    pub fn check_for_new_data(&mut self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.file_path)
            .with_context(|| format!("Failed to get metadata for {}", self.file_path.display()))?;

        let new_size = metadata.len();
        let new_inode = get_inode(&metadata);

        // Check if file was rotated (inode changed)
        if self.inode.is_some() && new_inode != self.inode {
            debug!("File rotation detected for {}", self.file_path.display());
            self.handle_file_rotation(new_size, new_inode);
            return Ok(0); // No new data from current position after rotation
        }

        // Check if file was truncated
        if new_size < self.current_size {
            debug!(
                "File truncation detected: {} -> {} bytes",
                self.current_size, new_size
            );
            self.current_position = 0;
            self.current_size = new_size;
            self.inode = new_inode;
            return Ok(new_size);
        }

        // Normal case: file grew
        let new_bytes = new_size.saturating_sub(self.current_size);
        self.current_size = new_size;
        self.inode = new_inode;

        Ok(new_bytes)
    }

    /// Read new lines from the file since the last position
    pub fn read_new_lines(&mut self) -> Result<Vec<String>> {
        let mut file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open {}", self.file_path.display()))?;

        // Seek to our current position
        file.seek(SeekFrom::Start(self.current_position))
            .context("Failed to seek to current position")?;

        let mut reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut line = String::new();

        // Read lines from current position to end of file
        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .context("Failed to read line from file")?;

            if bytes_read == 0 {
                break; // End of file
            }

            // Remove trailing newline
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }

            if !line.is_empty() {
                lines.push(line.clone());
            }

            self.current_position += bytes_read as u64;
        }

        debug!(
            "Read {} new lines from {}",
            lines.len(),
            self.file_path.display()
        );

        Ok(lines)
    }

    /// Handle file rotation scenario
    fn handle_file_rotation(&mut self, new_size: u64, new_inode: Option<u64>) {
        debug!("Handling file rotation, resetting position to 0");
        self.current_position = 0;
        self.current_size = new_size;
        self.inode = new_inode;
    }

    /// Get the current file position
    #[allow(dead_code)]
    pub fn current_position(&self) -> u64 {
        self.current_position
    }

    /// Get the current file size
    #[allow(dead_code)]
    pub fn current_size(&self) -> u64 {
        self.current_size
    }
}

/// Get file inode number for rotation detection (Unix-like systems)
#[cfg(unix)]
fn get_inode(metadata: &std::fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt as _;
    // Unix systems always have inodes, but we return Option for API consistency
    if metadata.len() == 0 && metadata.ino() == 0 {
        // Handle edge case of empty/invalid file
        None
    } else {
        Some(metadata.ino())
    }
}

/// Windows doesn't have inodes, so we use file index instead
#[cfg(windows)]
fn get_inode(metadata: &std::fs::Metadata) -> Option<u64> {
    use std::os::windows::fs::MetadataExt;
    // Use file index as a substitute for inode
    Some(metadata.file_index().unwrap_or(0))
}

/// For other platforms, we can't detect rotation reliably
#[cfg(not(any(unix, windows)))]
fn get_inode(_metadata: &std::fs::Metadata) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[test]
    fn test_position_tracker_new_file() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "initial line")?;
        temp_file.flush()?;

        let tracker = PositionTracker::new(temp_file.path())?;

        // Should start at end of file
        assert!(tracker.current_position() > 0);
        assert_eq!(tracker.current_position(), tracker.current_size());

        Ok(())
    }

    #[test]
    fn test_read_new_lines() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "line 1")?;
        temp_file.flush()?;

        let mut tracker = PositionTracker::new(temp_file.path())?;

        // Add new content
        writeln!(temp_file, "line 2")?;
        writeln!(temp_file, "line 3")?;
        temp_file.flush()?;

        // Check for new data
        let new_bytes = tracker.check_for_new_data()?;
        assert!(new_bytes > 0);

        // Read new lines
        let lines = tracker.read_new_lines()?;
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 2");
        assert_eq!(lines[1], "line 3");

        Ok(())
    }
}
