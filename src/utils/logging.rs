use std::io;
use std::io::Write;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::Local;
use console::{style, Color};
use log;

/// A simple logger
pub struct Logger;

lazy_static! {
    static ref MAX_LEVEL: AtomicUsize =
        AtomicUsize::new(unsafe { mem::transmute(log::LevelFilter::Warn) });
}

pub fn max_level() -> log::LevelFilter {
    unsafe { mem::transmute(MAX_LEVEL.load(Ordering::Relaxed)) }
}

pub fn set_max_level(level: log::LevelFilter) {
    MAX_LEVEL.store(unsafe { mem::transmute(level) }, Ordering::Relaxed);
}

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
            }).dim(),
        );

        writeln!(io::stderr(), "{}", msg).ok();
    }

    fn flush(&self) {}
}
