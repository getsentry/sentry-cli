use std::io;
use std::mem;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use log;
use console::{style, Color};

/// A simple logger
pub struct Logger;

lazy_static! {
    static ref MAX_LEVEL: AtomicUsize = AtomicUsize::new(
        unsafe { mem::transmute(log::LevelFilter::Warn) });
}

pub fn max_level() -> log::LevelFilter {
    unsafe { mem::transmute(MAX_LEVEL.load(Ordering::Relaxed)) }
}

pub fn set_max_level(level: log::LevelFilter) {
    MAX_LEVEL.store(unsafe { mem::transmute(level) }, Ordering::Relaxed);
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.metadata().level();
        let msg = format!("[{}] {} {}", level, record.target(), record.args());
        let styled = style(msg).fg(match level {
            log::Level::Error | log::Level::Warn => Color::Red,
            log::Level::Info => Color::Cyan,
            log::Level::Debug | log::Level::Trace => Color::Yellow,
        });

        writeln!(io::stderr(), "{}", styled).ok();
    }

    fn flush(&self) {}
}
