use std::io;
use std::io::Write;

use log;
use console::{style, Color};

/// A simple logger
pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
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
