use console::{style, Term};
use std::env;
use std::ops::Deref;
use std::sync::Arc;

use crate::utils::logging;

pub use indicatif::{ProgressDrawTarget, ProgressStyle};

pub fn is_progress_bar_visible() -> bool {
    env::var("SENTRY_NO_PROGRESS_BAR") != Ok("1".into())
}

pub struct ProgressBar {
    inner: Arc<indicatif::ProgressBar>,
}

impl ProgressBar {
    pub fn new(len: u64) -> Self {
        if is_progress_bar_visible() {
            indicatif::ProgressBar::new(len).into()
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

    pub fn finish_and_clear(&self) {
        self.inner.finish_and_clear();
        logging::set_progress_bar(None);
    }

    pub fn set_draw_target(&self, target: ProgressDrawTarget) {
        if is_progress_bar_visible() {
            self.inner.set_draw_target(target);
        }
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

pub fn make_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_draw_target(ProgressDrawTarget::to_term(Term::stdout(), None));
    pb.set_style(ProgressStyle::default_bar().template(&format!(
        "{} {{msg}}\n{{wide_bar}} {{pos}}/{{len}}",
        style(">").cyan()
    )));
    pb
}
