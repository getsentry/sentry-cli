use std::io;
use std::io::Write;

use log;
use console::{style, Color};

/// A simple logger
pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level() <= log::max_log_level()
    }

    fn log(&self, record: &log::LogRecord) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = record.metadata().level();
        let msg = format!("[{}] {} {}", level, record.target(), record.args());
        writeln!(io::stderr(), "{}", style(msg).fg(
            match level {
                log::LogLevel::Error | log::LogLevel::Warn => Color::Red,
                log::LogLevel::Info => Color::Cyan,
                log::LogLevel::Debug | log::LogLevel::Trace => Color::Yellow,
            }
        )).ok();
    }
}
