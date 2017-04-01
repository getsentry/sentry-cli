//! Provides sourcemap validation functionality.
use std::fs;
use std::io;
use std::fmt;
use std::env;
use std::io::{Read, Write};
use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use crates::term;
use crates::url::Url;
use crates::sourcemap;
use crates::might_be_minified;

use api::{Api, FileContents};

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
            }
            Err(x) => fail!(x),
        }
    } else {
        Ok(Url::parse(base_url)?.join(url)?.to_string())
    }
}

fn split_url(url: &str) -> (Option<&str>, &str, Option<&str>) {
    let mut part_iter = url.rsplitn(2, '/');
    let (filename, ext) = part_iter.next()
        .map(|x| {
            let mut fn_iter = x.splitn(2, '.');
            (fn_iter.next(), fn_iter.next())
        })
        .unwrap_or((None, None));
    let path = part_iter.next();
    (path, filename.unwrap_or(""), ext)
}

fn unsplit_url(path: Option<&str>, basename: &str, ext: Option<&str>) -> String {
    let mut rv = String::new();
    if let Some(path) = path {
        rv.push_str(path);
        rv.push('/');
    }
    rv.push_str(basename);
    if let Some(ext) = ext {
        rv.push('.');
        rv.push_str(ext);
    }
    rv
}

pub fn get_sourcemap_reference_from_headers<'a, I: Iterator<Item = (&'a String, &'a String)>>
    (headers: I)
     -> Option<&'a str> {
    for (k, v) in headers {
        let ki = &k.to_lowercase();
        if ki == "sourcemap" || ki == "x-sourcemap" {
            return Some(v.as_str());
        }
    }
    None
}


fn find_sourcemap_reference(sourcemaps: &HashSet<String>, min_url: &str) -> Result<String> {
    // if there is only one sourcemap in total we just assume that's the one.
    // We just need to make sure that we fix up the reference if we need to
    // (eg: ~/ -> /).
    if sourcemaps.len() == 1 {
        return Ok(sourcemap::make_relative_path(
            min_url, sourcemaps.iter().next().unwrap()));
    }

    let map_ext = "map";
    let (path, basename, ext) = split_url(min_url);

    // foo.min.js -> foo.map
    if sourcemaps.contains(&unsplit_url(path, basename, Some("map"))) {
        return Ok(unsplit_url(None, basename, Some("map")));
    }

    if let Some(ext) = ext.as_ref() {
        // foo.min.js -> foo.min.js.map
        let new_ext = format!("{}.{}", ext, map_ext);
        if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
            return Ok(unsplit_url(None, basename, Some(&new_ext)));
        }

        // foo.min.js -> foo.js.map
        if ext.starts_with("min.") {
            let new_ext = format!("{}.{}", &ext[4..], map_ext);
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(unsplit_url(None, basename, Some(&new_ext)));
            }
        }

        // foo.min.js -> foo.min.map
        let mut parts: Vec<_> = ext.split('.').collect();
        if parts.len() > 1 {
            let parts_len = parts.len();
            parts[parts_len - 1] = &map_ext;
            let new_ext = parts.join(".");
            println!("{:?}", unsplit_url(path, basename, Some(&new_ext)));
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(unsplit_url(None, basename, Some(&new_ext)));
            }
        }
    }

    fail!("Could not auto-detect referenced sourcemap for {}.",
          min_url);
}


#[derive(PartialEq)]
enum SourceType {
    Script,
    MinifiedScript,
    SourceMap,
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SourceType::Script => write!(f, "script"),
            SourceType::MinifiedScript => write!(f, "minified script"),
            SourceType::SourceMap => write!(f, "sourcemap"),
        }
    }
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
    contents: String,
    ty: SourceType,
    skip_upload: bool,
    headers: Vec<(String, String)>,
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
                LogLevel::Error => {
                    term.fg(term::color::RED).ok();
                }
                LogLevel::Warning => {
                    term.fg(term::color::YELLOW).ok();
                }
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
    pub fn add(&mut self, url: &str, path: &Path) -> Result<()> {
        let mut f = fs::File::open(&path)?;
        let mut contents = String::new();
        try!(f.read_to_string(&mut contents));
        let ty = if sourcemap::is_sourcemap_slice(contents.as_bytes()) {
            SourceType::SourceMap
        } else if path.file_name()
            .and_then(|x| x.to_str())
            .map(|x| x.contains(".min."))
            .unwrap_or(false) ||
                           might_be_minified::analyze_str(&contents).is_likely_minified() {
            SourceType::MinifiedScript
        } else {
            SourceType::Script
        };

        self.sources.insert(url.to_owned(),
                            Source {
                                url: url.to_owned(),
                                file_path: path.to_path_buf(),
                                contents: contents,
                                ty: ty,
                                skip_upload: false,
                                headers: vec![],
                            });
        Ok(())
    }

    fn validate_script(&self, source: &Source) -> Result<()> {
        let reference = sourcemap::locate_sourcemap_reference_slice(source.contents.as_bytes())?;
        if let sourcemap::SourceMapRef::LegacyRef(_) = reference {
            self.log.warn(source, "encountered a legacy reference".into());
        }
        if let Some(url) = reference.get_url() {
            let full_url = join_url(&source.url, url)?;
            self.log.info(source, format!("sourcemap at {}", full_url));
        } else if source.ty == SourceType::MinifiedScript {
            self.log.error(source, "missing sourcemap!".into());
        } else {
            self.log.warn(source, "no sourcemap reference".into());
        }
        Ok(())
    }

    fn validate_sourcemap(&self, source: &Source) -> Result<()> {
        match sourcemap::decode_slice(source.contents.as_bytes())? {
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
            }
            sourcemap::DecodedMap::Index(_) => {
                self.log.warn(source,
                              "encountered indexed sourcemap. We cannot validate those.".into());
            }
        }
        Ok(())
    }

    /// Validates all sources within.
    pub fn validate_all(&self) -> Result<()> {
        let mut sources: Vec<_> = self.sources.iter().map(|x| x.1).collect();
        sources.sort_by_key(|x| &x.url);
        let mut failed = false;

        for source in sources.iter() {
            match source.ty {
                SourceType::Script |
                SourceType::MinifiedScript => {
                    if let Err(err) = self.validate_script(&source) {
                        self.log.error(&source, format!("failed to process: {}", err));
                        failed = true;
                    }
                }
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
    pub fn rewrite(&mut self, prefixes: &[&str]) -> Result<()> {
        for (_, source) in self.sources.iter_mut() {
            if source.ty != SourceType::SourceMap {
                continue;
            }
            let options = sourcemap::RewriteOptions {
                load_local_source_contents: true,
                strip_prefixes: prefixes,
                ..Default::default()
            };
            let sm = match sourcemap::decode_slice(source.contents.as_bytes())? {
                sourcemap::DecodedMap::Regular(sm) => sm.rewrite(&options)?,
                sourcemap::DecodedMap::Index(smi) => smi.flatten_and_rewrite(&options)?,
            };
            let mut new_source: Vec<u8> = Vec::new();
            sm.to_writer(&mut new_source)?;
            source.contents = String::from_utf8(new_source)?;
        }
        Ok(())
    }

    /// Adds sourcemap references to all minified files
    pub fn add_sourcemap_references(&mut self) -> Result<()> {
        let sourcemaps = HashSet::from_iter(self.sources
            .iter()
            .map(|x| x.1)
            .filter(|x| x.ty == SourceType::SourceMap)
            .map(|x| x.url.to_string()));
        for (_, source) in self.sources.iter_mut() {
            if source.ty != SourceType::MinifiedScript {
                continue;
            }
            // we silently ignore when we can't find a sourcemap. Maybwe we should
            // log this.
            match find_sourcemap_reference(&sourcemaps, &source.url) {
                Ok(target_url) => {
                    source.headers.push(("Sourcemap".to_string(), target_url));
                }
                Err(err) => {
                    self.log.warn(source,
                                  format!("could not determine a sourcemap reference ({})", err));
                }
            }
        }
        Ok(())
    }

    /// Uploads all files
    pub fn upload(&self, api: &Api, org: &str, project: &str, release: &str) -> Result<()> {
        let here = env::current_dir()?;
        for (_, source) in self.sources.iter() {
            if source.skip_upload {
                continue;
            }
            let display_path = here.strip_prefix(&here);
            println!("{} -> {} [{}]",
                     display_path.as_ref()
                         .unwrap_or(&source.file_path.as_path())
                         .display(),
                     &source.url,
                     source.ty);
            if let Some(artifact) = api.upload_release_file(org,
                                     project,
                                     &release,
                                     FileContents::FromBytes(source.contents.as_bytes()),
                                     &source.url,
                                     Some(source.headers.as_slice()))? {
                println!("  {}  ({} bytes)", artifact.sha1, artifact.size);
            } else {
                println!("  already present");
            }
            if source.ty == SourceType::MinifiedScript {
                if let Some(sm_ref) = get_sourcemap_reference_from_headers(source.headers
                    .iter()
                    .map(|&(ref k, ref v)| (k, v))) {
                    println!("  -> sourcemap: {}", sm_ref);
                }
            }
        }
        Ok(())
    }
}


#[test]
fn test_split_url() {
    assert_eq!(split_url("/foo.js"), (Some(""), "foo", Some("js")));
    assert_eq!(split_url("foo.js"), (None, "foo", Some("js")));
    assert_eq!(split_url("foo"), (None, "foo", None));
    assert_eq!(split_url("/foo"), (Some(""), "foo", None));
}

#[test]
fn test_unsplit_url() {
    assert_eq!(&unsplit_url(Some(""), "foo", Some("js")), "/foo.js");
    assert_eq!(&unsplit_url(None, "foo", Some("js")), "foo.js");
    assert_eq!(&unsplit_url(None, "foo", None), "foo");
    assert_eq!(&unsplit_url(Some(""), "foo", None), "/foo");
}
