use parking_lot::RwLock;
use std::env;
use std::io::IsTerminal as _;
use std::sync::Arc;
use std::time::Instant;

use crate::utils::logging;

pub use indicatif::ProgressStyle;

/// By default, use the progress bar when we can detect we're running in a terminal.
/// `SENTRY_NO_PROGRESS_BAR` takes precedence and can be used to disable the progress bar
/// regardless of whether or not we're in a terminal.
pub fn use_progress_bar() -> bool {
    if env::var("SENTRY_NO_PROGRESS_BAR") == Ok("1".into()) {
        return false;
    }
    std::io::stdout().is_terminal() && std::io::stderr().is_terminal()
}

/// Wrapper that optionally holds a progress bar.
/// If there's a progress bar, forward calls to it.
/// Otherwise, log messages normally.
pub struct ProgressBar {
    inner: Option<Arc<indicatif::ProgressBar>>,
    start: Instant,
}

impl ProgressBar {
    pub fn new(len: usize) -> Self {
        if use_progress_bar() {
            Self::from_indicatif(indicatif::ProgressBar::new(len as u64))
        } else {
            Self::no_progress_bar()
        }
    }

    pub fn new_spinner() -> Self {
        if use_progress_bar() {
            Self::from_indicatif(indicatif::ProgressBar::new_spinner())
        } else {
            Self::no_progress_bar()
        }
    }

    fn from_indicatif(pb: indicatif::ProgressBar) -> Self {
        let inner = Arc::new(pb);
        logging::set_progress_bar(Some(Arc::downgrade(&inner)));
        ProgressBar {
            inner: Some(inner),
            start: Instant::now(),
        }
    }

    fn no_progress_bar() -> Self {
        ProgressBar {
            inner: None,
            start: Instant::now(),
        }
    }

    pub fn finish_with_duration(&self, op: &str) {
        let dur = self.start.elapsed();
        // We could use `dur.as_secs_f64()`, but its unnecessarily precise (micros). Millis are enough for our purpose.
        let msg = format!("{op} completed in {}s", dur.as_millis() as f64 / 1000.0);

        if let Some(inner) = &self.inner {
            let progress_style = ProgressStyle::default_bar().template("{prefix:.dim} {msg}");
            inner.set_style(progress_style);
            inner.set_prefix(">");
            inner.finish_with_message(&msg);
            logging::set_progress_bar(None);
        } else {
            log::info!("{msg}");
        }
    }

    pub fn finish_and_clear(&self) {
        if let Some(inner) = &self.inner {
            inner.finish_and_clear();
            logging::set_progress_bar(None);
        }
    }

    pub fn set_style(&self, style: ProgressStyle) {
        if let Some(inner) = &self.inner {
            inner.set_style(style);
        }
    }

    pub fn set_message<S: AsRef<str>>(&self, msg: S) {
        if let Some(inner) = &self.inner {
            inner.set_message(msg.as_ref());
        } else {
            log::debug!("{}", msg.as_ref());
        }
    }

    pub fn tick(&self) {
        if let Some(inner) = &self.inner {
            inner.tick();
        }
    }

    pub fn set_prefix<S: AsRef<str>>(&self, prefix: S) {
        if let Some(inner) = &self.inner {
            inner.set_prefix(prefix.as_ref());
        }
    }

    pub fn set_position(&self, pos: u64) {
        if let Some(inner) = &self.inner {
            inner.set_position(pos);
        }
    }

    pub fn inc(&self, delta: u64) {
        if let Some(inner) = &self.inner {
            inner.inc(delta);
        }
    }

    pub fn enable_steady_tick(&self, interval: u64) {
        if let Some(inner) = &self.inner {
            inner.enable_steady_tick(interval);
        }
    }
}

#[derive(Clone)]
pub enum ProgressBarMode {
    Disabled,
    Request,
    #[cfg(not(feature = "managed"))]
    Response,
    Shared((Arc<ProgressBar>, u64, usize, Arc<RwLock<Vec<u64>>>)),
}

impl ProgressBarMode {
    /// Returns if progress bars are generally enabled.
    pub fn active(&self) -> bool {
        !matches!(*self, ProgressBarMode::Disabled)
    }

    /// Returns whether a progress bar should be displayed during upload.
    pub fn request(&self) -> bool {
        matches!(*self, ProgressBarMode::Request)
    }

    /// Returns whether a progress bar should be displayed during download.
    pub fn response(&self) -> bool {
        #[cfg(not(feature = "managed"))]
        let rv = matches!(*self, ProgressBarMode::Response);

        #[cfg(feature = "managed")]
        let rv = false;

        rv
    }
}
