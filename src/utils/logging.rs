use std::sync::{Arc, Weak};

use chrono::Local;
use console::{style, Color};
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use log::max_level;
use parking_lot::RwLock;

lazy_static! {
    static ref PROGRESS_BAR: RwLock<Option<Weak<ProgressBar>>> = RwLock::new(None);
}

/// A simple logger
pub struct Logger;

impl Logger {
    fn get_actual_level(&self, metadata: &log::Metadata) -> log::Level {
        let mut level = metadata.level();
        if level == log::Level::Debug
            && (metadata.target() == "tokio_reactor"
                || metadata.target().starts_with("hyper::proto::"))
        {
            level = log::Level::Trace;
        }
        level
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.get_actual_level(metadata) <= max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = self.get_actual_level(record.metadata());
        let (level_name, level_color) = match level {
            log::Level::Error => ("ERROR", Color::Red),
            log::Level::Warn => ("WARN ", Color::Red),
            log::Level::Info => ("INFO ", Color::Cyan),
            log::Level::Debug => ("DEBUG", Color::Yellow),
            log::Level::Trace => ("TRACE", Color::Magenta),
        };
        let short_target = record.target().split("::").next().unwrap_or("");
        let msg = format!(
            "{} {} {}{}",
            style(format!("  {}  ", level_name)).bg(level_color).black(),
            style(Local::now()).dim(),
            style(record.args()),
            style(if short_target != "sentry_cli" {
                format!("  (from {})", short_target)
            } else {
                "".to_string()
            })
            .dim(),
        );

        if should_skip_log(record) {
            return;
        }

        if let Some(pb) = get_progress_bar() {
            pb.println(msg);
        } else {
            eprintln!("{}", msg);
        }
    }

    fn flush(&self) {}
}

fn should_skip_log(record: &log::Record) -> bool {
    let level = record.metadata().level();
    let target = record.target();

    // We want to filter everything that is non-error from `goblin` crate,
    // as `symbolicator` is responsible for making sure all warnings are handled correctly.
    if target.starts_with("goblin") && level != log::LevelFilter::Error {
        return true;
    }

    false
}

pub fn set_progress_bar(pb: Option<Weak<ProgressBar>>) {
    *PROGRESS_BAR.write() = pb;
}

fn get_progress_bar() -> Option<Arc<ProgressBar>> {
    PROGRESS_BAR.read().as_ref()?.upgrade()
}
