use std::env;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

use crate::utils::logging;

pub use indicatif::{ProgressDrawTarget, ProgressStyle};

pub fn is_progress_bar_visible() -> bool {
    env::var("SENTRY_NO_PROGRESS_BAR") != Ok("1".into())
}

pub struct ProgressBar {
    inner: Arc<indicatif::ProgressBar>,
    start: Instant,
}

impl ProgressBar {
    pub fn new(len: usize) -> Self {
        if is_progress_bar_visible() {
            indicatif::ProgressBar::new(len as u64).into()
        } else {
            Self::hidden()
        }
    }

    pub fn new_spinner() -> Self {
        if is_progress_bar_visible() {
            indicatif::ProgressBar::new_spinner().into()
        } else {
            Self::hidden()
        }
    }

    pub fn hidden() -> Self {
        indicatif::ProgressBar::hidden().into()
    }

    pub fn finish(&self) {
        self.inner.finish();
        logging::set_progress_bar(None);
    }

    pub fn finish_with_message(&self, msg: &str) {
        self.inner.finish_with_message(msg);
        logging::set_progress_bar(None);
    }

    pub fn finish_with_duration(&self, op: &str) {
        let dur = self.start.elapsed();
        // We could use `dur.as_secs_f64()`, but its unnecessarily precise (micros). Millis are enough for our purpose.
        let msg = format!("{} completed in {}s", op, dur.as_millis() as f64 / 1000.0);
        let progress_style = ProgressStyle::default_bar().template("{prefix:.dim} {msg}");
        self.inner.set_style(progress_style);
        self.inner.set_prefix(">");
        self.inner.finish_with_message(&msg);
        logging::set_progress_bar(None);
    }

    pub fn finish_and_clear(&self) {
        self.inner.finish_and_clear();
        logging::set_progress_bar(None);
    }
}

impl From<indicatif::ProgressBar> for ProgressBar {
    fn from(pb: indicatif::ProgressBar) -> Self {
        let inner = Arc::new(pb);
        logging::set_progress_bar(Some(Arc::downgrade(&inner)));
        ProgressBar {
            inner,
            start: Instant::now(),
        }
    }
}

impl Deref for ProgressBar {
    type Target = indicatif::ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
