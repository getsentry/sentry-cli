use std::ops::Deref;
use std::sync::Arc;

use crate::utils::logging;

pub use indicatif::{ProgressDrawTarget, ProgressStyle};

pub struct ProgressBar {
    inner: Arc<indicatif::ProgressBar>,
}

impl ProgressBar {
    pub fn new(len: u64) -> Self {
        indicatif::ProgressBar::new(len).into()
    }

    pub fn hidden() -> Self {
        indicatif::ProgressBar::hidden().into()
    }

    pub fn new_spinner() -> Self {
        indicatif::ProgressBar::new_spinner().into()
    }

    pub fn finish(&self) {
        self.inner.finish();
        logging::set_progress_bar(None);
    }

    pub fn finish_with_message(&self, msg: &str) {
        self.inner.finish_with_message(msg);
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
        ProgressBar { inner }
    }
}

impl Deref for ProgressBar {
    type Target = indicatif::ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
