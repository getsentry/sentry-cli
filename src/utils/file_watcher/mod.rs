use anyhow::{Context as _, Result};
use log::{debug, warn};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

pub use self::position_tracker::PositionTracker;

mod position_tracker;

/// Events that can occur during file watching
#[derive(Debug, Clone)]

pub enum FileEvent {
    /// New data has been written to the file
    DataWritten,

    /// File was moved or renamed
    Moved,
    /// File was deleted
    Deleted,
    /// File was created (for handling log rotation)
    Created,
}

/// Cross-platform file watcher for monitoring log files
pub struct LogFileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    file_path: std::path::PathBuf,
}

impl LogFileWatcher {
    /// Create a new file watcher for the specified file
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        let (tx, receiver) = mpsc::channel();

        // Configure watcher with appropriate settings for log monitoring
        let config = Config::default()
            .with_poll_interval(Duration::from_millis(500))
            .with_compare_contents(false); // We only care about size changes

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.send(res) {
                    warn!("Failed to send file system event: {}", e);
                }
            },
            config,
        )?;

        // Watch the file directly if possible, otherwise watch its parent directory
        let watch_path = if file_path.is_file() {
            &file_path
        } else if let Some(parent) = file_path.parent() {
            parent
        } else {
            &file_path
        };

        watcher
            .watch(watch_path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch path: {}", watch_path.display()))?;

        debug!("Started watching file: {}", file_path.display());

        Ok(LogFileWatcher {
            _watcher: watcher,
            receiver,
            file_path,
        })
    }

    /// Check for file events with a timeout
    /// Returns None if no events occur within the timeout period
    pub fn check_events(&self, timeout: Duration) -> Result<Option<FileEvent>> {
        match self.receiver.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                debug!("File system event: {:?}", event);
                Ok(self.process_event(event))
            }
            Ok(Err(e)) => {
                warn!("File system notification error: {}", e);
                Ok(None)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("File watcher channel disconnected");
            }
        }
    }

    /// Process a file system event and convert it to a LogEvent
    fn process_event(&self, event: Event) -> Option<FileEvent> {
        // Only process events for our target file
        if !event.paths.iter().any(|p| p == &self.file_path) {
            return None;
        }

        match event.kind {
            EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                // File content was modified
                if let Ok(_metadata) = std::fs::metadata(&self.file_path) {
                    Some(FileEvent::DataWritten)
                } else {
                    // File might have been deleted
                    Some(FileEvent::Deleted)
                }
            }
            EventKind::Modify(notify::event::ModifyKind::Metadata(_)) => {
                // File metadata changed, check if size changed
                if let Ok(_metadata) = std::fs::metadata(&self.file_path) {
                    Some(FileEvent::DataWritten)
                } else {
                    None
                }
            }
            EventKind::Remove(_) => Some(FileEvent::Deleted),
            EventKind::Create(_) => Some(FileEvent::Created),
            EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::To)) => {
                Some(FileEvent::Moved)
            }
            _ => {
                debug!("Ignoring file system event: {:?}", event.kind);
                None
            }
        }
    }
}

/// Handle graceful shutdown signals
pub fn setup_signal_handlers() -> Result<Receiver<()>> {
    let (tx, rx) = mpsc::channel();

    // Handle SIGINT (Ctrl+C) and SIGTERM
    ctrlc::set_handler(move || {
        debug!("Received shutdown signal");
        if let Err(e) = tx.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }
    })
    .context("Failed to set signal handler")?;

    Ok(rx)
}
