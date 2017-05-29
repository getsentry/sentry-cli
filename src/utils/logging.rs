use std::io;
use std::io::Write;

use log;
use console::{style, Color};

/// A simple logger
pub struct Logger;

impl Logger {
    pub fn get_real_level(&self, metadata: &log::LogMetadata) -> log::LogLevel {
        // upgrade debug -> trace for mach_object as its too spammy otherwise
        if metadata.level() == log::LogLevel::Debug &&
           metadata.target().starts_with("mach_object::") {
            return log::LogLevel::Trace;
        }
        metadata.level()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        self.get_real_level(metadata) <= log::max_log_level()
    }

    fn log(&self, record: &log::LogRecord) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = self.get_real_level(record.metadata());
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
