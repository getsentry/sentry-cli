//! Provides sourcemap validation functionality.
use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use term;
use url::Url;
use sourcemap;

use prelude::*;

fn join_url(base_url: &str, url: &str) -> Result<String> {
    if base_url.starts_with("~/") {
        match Url::parse(&format!("http://{}", base_url))?.join(url) {
            Ok(url) => {
                let rv = url.to_string();
                if rv.starts_with("http://~/") {
                    Ok(format!("~/{}", &rv[9..]))
                } else {
                    Ok(rv)
                }
            },
            Err(x) => fail!(x)
        }
    } else {
        Ok(Url::parse(base_url)?.join(url)?.to_string())
    }
}


#[derive(PartialEq)]
enum SourceType {
    Script,
    SourceMap,
}

#[derive(PartialEq, Debug)]
enum LogLevel {
    Info,
    Warning,
    Error,
}

impl LogLevel {
    fn is_insignificant(&self) -> bool {
        *self == LogLevel::Info
    }
}

struct Source {
    url: String,
    file_path: PathBuf,
    ty: SourceType,
}

/// Validates sourcemaps.
pub struct SourceMapValidator {
    sources: HashMap<String, Source>,
    verbose: bool,
}

struct Log {
    pub failed: bool,
    last_source: Option<String>,
    verbose: bool,
}

impl Log {
    pub fn new(verbose: bool) -> Log {
        Log {
            failed: false,
            last_source: None,
            verbose: verbose,
        }
    }

    pub fn log(&mut self, source: &Source, level: LogLevel, message: String) {
        let mut term = term::stderr().unwrap();
        match level {
            LogLevel::Error => { term.fg(term::color::RED).ok(); }
            LogLevel::Warning => { term.fg(term::color::YELLOW).ok(); }
            LogLevel::Info => {}
        }
        if self.last_source.as_ref() != Some(&source.url) {
            self.last_source = Some(source.url.clone());
            writeln!(term, "  {}", source.url).ok();
        }
        if level.is_insignificant() && !self.verbose {
            return;
        }
        writeln!(term, "    {:?}: {}", level, &message).ok();
        if level == LogLevel::Error {
            self.failed = true;
        }
        term.reset().ok();
    }

    pub fn error(&mut self, source: &Source, message: String) {
        self.log(source, LogLevel::Error, message);
    }

    pub fn warn(&mut self, source: &Source, message: String) {
        self.log(source, LogLevel::Warning, message);
    }

    pub fn info(&mut self, source: &Source, message: String) {
        self.log(source, LogLevel::Info, message);
    }
}

impl SourceMapValidator {
    /// Creates a new sourcemap validator.  If it's set to verbose
    /// it prints the progress to stdout.
    pub fn new(verbose: bool) -> SourceMapValidator {
        SourceMapValidator {
            sources: HashMap::new(),
            verbose: verbose,
        }
    }

    /// Adds a file for consideration.
    pub fn consider_file(&mut self, path: &Path, url: &str) -> bool {
        let ty = match path.extension().and_then(|x| x.to_str()) {
            Some("js") => SourceType::Script,
            Some("map") => SourceType::SourceMap,
            _ => { return false; }
        };
        self.sources.insert(url.to_owned(), Source {
            url: url.to_owned(),
            file_path: path.to_path_buf(),
            ty: ty,
        });
        true
    }

    fn validate_script(&self, log: &mut Log, source: &Source) -> Result<()> {
        let f = fs::File::open(&source.file_path)?;
        let reference = sourcemap::locate_sourcemap_reference(&f)?;
        if let sourcemap::SourceMapRef::LegacyRef(_) = reference {
            log.warn(source, "encountered a legacy reference".into());
        }
        if let Some(url) = reference.get_url() {
            let full_url = join_url(&source.url, url)?;
            log.info(source, format!("sourcemap at {}", full_url));
        } else if source.url.ends_with(".min.js") {
            log.error(source, "missing sourcemap!".into());
        } else {
            log.warn(source, "no sourcemap reference".into());
        }
        Ok(())
    }

    fn validate_sourcemap(&self, log: &mut Log, source: &Source) -> Result<()> {
        let f = fs::File::open(&source.file_path)?;
        match sourcemap::decode(&f)? {
            sourcemap::DecodedMap::Regular(sm) => {
                for idx in 0..sm.get_source_count() {
                    let source_url = sm.get_source(idx).unwrap_or("??");
                    if sm.get_source_contents(idx).is_some() ||
                       self.sources.get(source_url).is_some() {
                        log.info(source, format!("found source ({})", source_url));
                    } else {
                        log.warn(source, format!("missing sourcecode ({})", source_url));
                    }
                }
            },
            sourcemap::DecodedMap::Index(_) => {
                log.warn(source, "encountered indexed sourcemap. We 
                         cannot validate those.".into());
            }
        }
        Ok(())
    }

    /// Validates all sources within.
    pub fn validate_sources(&self) -> Result<()> {
        let mut log = Log::new(self.verbose);
        let mut sources : Vec<_> = self.sources.iter().map(|x| x.1).collect();
        sources.sort_by_key(|x| &x.url);

        for source in sources.iter() {
            match source.ty {
                SourceType::Script => {
                    if let Err(err) = self.validate_script(&mut log, &source) {
                        log.error(&source, format!("failed to process: {}", err));
                    }
                },
                SourceType::SourceMap => {
                    if let Err(err) = self.validate_sourcemap(&mut log, &source) {
                        log.error(&source, format!("failed to process: {}", err));
                    }
                }
            }
        }
        if log.failed {
            fail!("Encountered problems when validating sourcemaps.");
        }
        println!("All Good!");
        Ok(())
    }
}
