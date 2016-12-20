//! Provides sourcemap validation functionality.
use std::fs;
use std::io;
use std::io::Write;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use term;
use url::Url;
use api::Api;
use sourcemap;
use sourcemap::is_sourcemap;

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

struct Log {
    last_source: RefCell<Option<String>>,
    verbose: bool,
}

pub struct SourceMapProcessor {
    sources: HashMap<String, Source>,
    log: Log,
}

impl Log {
    pub fn new(verbose: bool) -> Log {
        Log {
            last_source: RefCell::new(None),
            verbose: verbose,
        }
    }

    pub fn log(&self, source: &Source, level: LogLevel, message: String) {
        let mut out_term;
        let mut out_stderr;
        let mut w = if let Some(mut term) = term::stderr() {
            match level {
                LogLevel::Error => { term.fg(term::color::RED).ok(); }
                LogLevel::Warning => { term.fg(term::color::YELLOW).ok(); }
                LogLevel::Info => {}
            }
            out_term = term;
            &mut out_term as &mut Write
        } else {
            out_stderr = io::stderr();
            &mut out_stderr as &mut Write
        };

        {
            let mut last_source = self.last_source.borrow_mut();
            if last_source.as_ref() != Some(&source.url) {
                *last_source = Some(source.url.clone());
                writeln!(w, "  {}", source.url).ok();
            }
        }
        if level.is_insignificant() && !self.verbose {
            return;
        }
        writeln!(w, "    {:?}: {}", level, &message).ok();
        if let Some(mut term) = term::stderr() {
            term.reset().ok();
        }
    }

    pub fn error(&self, source: &Source, message: String) {
        self.log(source, LogLevel::Error, message);
    }

    pub fn warn(&self, source: &Source, message: String) {
        self.log(source, LogLevel::Warning, message);
    }

    pub fn info(&self, source: &Source, message: String) {
        self.log(source, LogLevel::Info, message);
    }
}

impl SourceMapProcessor {
    /// Creates a new sourcemap validator.  If it's set to verbose
    /// it prints the progress to stdout.
    pub fn new(verbose: bool) -> SourceMapProcessor {
        SourceMapProcessor {
            sources: HashMap::new(),
            log: Log::new(verbose),
        }
    }

    /// Adds a new file for processing.
    pub fn add(&mut self, url: &str, local_path: &Path, path: &Path) -> Result<()> {
        let mut f = fs::File::open(&path)?;
        let ty = if is_sourcemap(&mut f) {
            SourceType::SourceMap
        } else {
            SourceType::Script
        };
        self.sources.insert(url.to_owned(), Source {
            url: url.to_owned(),
            file_path: path.to_path_buf(),
            ty: ty,
        });
        Ok(())
    }

    fn validate_script(&self, source: &Source) -> Result<()> {
        let f = fs::File::open(&source.file_path)?;
        let reference = sourcemap::locate_sourcemap_reference(&f)?;
        if let sourcemap::SourceMapRef::LegacyRef(_) = reference {
            self.log.warn(source, "encountered a legacy reference".into());
        }
        if let Some(url) = reference.get_url() {
            let full_url = join_url(&source.url, url)?;
            self.log.info(source, format!("sourcemap at {}", full_url));
        } else if source.url.ends_with(".min.js") {
            self.log.error(source, "missing sourcemap!".into());
        } else {
            self.log.warn(source, "no sourcemap reference".into());
        }
        Ok(())
    }

    fn validate_sourcemap(&self, source: &Source) -> Result<()> {
        let f = fs::File::open(&source.file_path)?;
        match sourcemap::decode(&f)? {
            sourcemap::DecodedMap::Regular(sm) => {
                for idx in 0..sm.get_source_count() {
                    let source_url = sm.get_source(idx).unwrap_or("??");
                    if sm.get_source_contents(idx).is_some() ||
                       self.sources.get(source_url).is_some() {
                        self.log.info(source, format!("found source ({})", source_url));
                    } else {
                        self.log.warn(source, format!("missing sourcecode ({})", source_url));
                    }
                }
            },
            sourcemap::DecodedMap::Index(_) => {
                self.log.warn(source, "encountered indexed sourcemap. We 
                              cannot validate those.".into());
            }
        }
        Ok(())
    }

    /// Validates all sources within.
    pub fn validate_all(&self) -> Result<()> {
        let mut sources : Vec<_> = self.sources.iter().map(|x| x.1).collect();
        sources.sort_by_key(|x| &x.url);
        let mut failed = false;

        for source in sources.iter() {
            match source.ty {
                SourceType::Script => {
                    if let Err(err) = self.validate_script(&source) {
                        self.log.error(&source, format!("failed to process: {}", err));
                        failed = true;
                    }
                },
                SourceType::SourceMap => {
                    if let Err(err) = self.validate_sourcemap(&source) {
                        self.log.error(&source, format!("failed to process: {}", err));
                        failed = true;
                    }
                }
            }
        }
        if failed {
            fail!("Encountered problems when validating sourcemaps.");
        }
        println!("All Good!");
        Ok(())
    }

    /// Automatically rewrite all sourcemaps.
    ///
    /// This inlines sources, flattens indexes and skips individual uploads.
    pub fn auto_rewrite(&mut self) -> Result<()> {
        Ok(())
    }

    /// Uploads all files
    pub fn upload(&self, _api: &Api) -> Result<()> {
        // for (url, local_path, path) in to_process {
        //     println!("{} -> {}", local_path.display(), url);
        //     if let Some(artifact) = api.upload_release_file(
        //         org, project, &release.version, &path, &url)? {
        //         println!("  {}  ({} bytes)", artifact.sha1, artifact.size);
        //     } else {
        //         println!("  already present");
        //     }
        // }
        Ok(())
    }
}
