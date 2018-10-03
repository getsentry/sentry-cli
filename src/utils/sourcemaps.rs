//! Provides sourcemap validation functionality.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Read;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::str;

use console::{style, Term};
use failure::Error;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use might_be_minified;
use sourcemap;
use url::Url;

use api::{Api, FileContents};
use utils::enc::decode_unknown_string;

fn make_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_draw_target(ProgressDrawTarget::to_term(Term::stdout(), None));
    pb.set_style(ProgressStyle::default_bar().template(&format!(
        "{} {{msg}}\n{{wide_bar}} {{pos}}/{{len}}",
        style(">").cyan()
    )));
    pb
}

fn is_likely_minified_js(code: &[u8]) -> bool {
    if let Ok(code_str) = decode_unknown_string(code) {
        might_be_minified::analyze_str(&code_str).is_likely_minified()
    } else {
        false
    }
}

fn join_url(base_url: &str, url: &str) -> Result<String, Error> {
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
            Err(x) => Err(Error::from(x).context("could not join URL").into()),
        }
    } else {
        Ok(Url::parse(base_url)?.join(url)?.to_string())
    }
}

fn split_url(url: &str) -> (Option<&str>, &str, Option<&str>) {
    let mut part_iter = url.rsplitn(2, '/');
    let (filename, ext) = part_iter
        .next()
        .map(|x| {
            let mut fn_iter = x.splitn(2, '.');
            (fn_iter.next(), fn_iter.next())
        }).unwrap_or((None, None));
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

pub fn get_sourcemap_reference_from_headers<'a, I: Iterator<Item = (&'a String, &'a String)>>(
    headers: I,
) -> Option<&'a str> {
    for (k, v) in headers {
        let ki = &k.to_lowercase();
        if ki == "sourcemap" || ki == "x-sourcemap" {
            return Some(v.as_str());
        }
    }
    None
}

fn guess_sourcemap_reference(sourcemaps: &HashSet<String>, min_url: &str) -> Result<String, Error> {
    // if there is only one sourcemap in total we just assume that's the one.
    // We just need to make sure that we fix up the reference if we need to
    // (eg: ~/ -> /).
    if sourcemaps.len() == 1 {
        return Ok(sourcemap::make_relative_path(
            min_url,
            sourcemaps.iter().next().unwrap(),
        ));
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
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(unsplit_url(None, basename, Some(&new_ext)));
            }
        }
    }

    bail!(
        "Could not auto-detect referenced sourcemap for {}.",
        min_url
    );
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum SourceType {
    Script,
    MinifiedScript,
    SourceMap,
}

#[derive(PartialEq, Debug)]
enum LogLevel {
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LogLevel::Warning => write!(f, "warning"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

struct Source {
    url: String,
    #[allow(unused)]
    file_path: PathBuf,
    contents: Vec<u8>,
    ty: SourceType,
    skip_upload: bool,
    headers: Vec<(String, String)>,
    messages: RefCell<Vec<(LogLevel, String)>>,
}

pub struct SourceMapProcessor {
    pending_sources: HashSet<(String, PathBuf)>,
    sources: HashMap<String, Source>,
}

impl Source {
    fn log(&self, level: LogLevel, msg: String) {
        self.messages.borrow_mut().push((level, msg));
    }

    fn warn(&self, msg: String) {
        self.log(LogLevel::Warning, msg);
    }

    fn error(&self, msg: String) {
        self.log(LogLevel::Error, msg);
    }

    pub fn get_sourcemap_ref_from_headers(&self) -> sourcemap::SourceMapRef {
        if let Some(sm_ref) =
            get_sourcemap_reference_from_headers(self.headers.iter().map(|&(ref k, ref v)| (k, v)))
        {
            sourcemap::SourceMapRef::Ref(sm_ref.to_string())
        } else {
            sourcemap::SourceMapRef::Missing
        }
    }

    pub fn get_sourcemap_ref_from_contents(&self) -> sourcemap::SourceMapRef {
        sourcemap::locate_sourcemap_reference_slice(&self.contents)
            .unwrap_or(sourcemap::SourceMapRef::Missing)
    }

    pub fn get_sourcemap_ref(&self) -> sourcemap::SourceMapRef {
        match self.get_sourcemap_ref_from_headers() {
            sourcemap::SourceMapRef::Missing => {}
            other => {
                return other;
            }
        }
        self.get_sourcemap_ref_from_contents()
    }
}

impl SourceMapProcessor {
    /// Creates a new sourcemap validator.
    pub fn new() -> SourceMapProcessor {
        SourceMapProcessor {
            pending_sources: HashSet::new(),
            sources: HashMap::new(),
        }
    }

    /// Adds a new file for processing.
    pub fn add(&mut self, url: &str, path: &Path) -> Result<(), Error> {
        self.pending_sources
            .insert((url.to_string(), path.to_path_buf()));
        Ok(())
    }

    fn flush_pending_sources(&mut self) -> Result<(), Error> {
        if self.pending_sources.is_empty() {
            return Ok(());
        }

        let pb = make_progress_bar(self.pending_sources.len() as u64);

        println!(
            "{} Analyzing {} sources",
            style(">").dim(),
            style(self.pending_sources.len()).yellow()
        );
        for (url, path) in self.pending_sources.drain() {
            pb.set_message(&url);
            let mut f = fs::File::open(&path)?;
            let mut contents: Vec<u8> = vec![];
            try!(f.read_to_end(&mut contents));
            let ty = if sourcemap::is_sourcemap_slice(&contents) {
                SourceType::SourceMap
            } else if path
                .file_name()
                .and_then(|x| x.to_str())
                .map(|x| x.contains(".min."))
                .unwrap_or(false)
                || is_likely_minified_js(&contents)
            {
                SourceType::MinifiedScript
            } else {
                SourceType::Script
            };

            self.sources.insert(
                url.clone(),
                Source {
                    url: url.clone(),
                    file_path: path.to_path_buf(),
                    contents,
                    ty,
                    skip_upload: false,
                    headers: vec![],
                    messages: RefCell::new(vec![]),
                },
            );
            pb.inc(1);
        }

        pb.finish_and_clear();

        Ok(())
    }

    fn validate_script(&self, source: &Source) -> Result<(), Error> {
        let sm_ref = source.get_sourcemap_ref();
        if let sourcemap::SourceMapRef::LegacyRef(_) = sm_ref {
            source.warn("encountered a legacy reference".into());
        }
        if let Some(url) = sm_ref.get_url() {
            let full_url = join_url(&source.url, url)?;
            info!("found sourcemap for {} at {}", &source.url, full_url);
        } else if source.ty == SourceType::MinifiedScript {
            source.error("missing sourcemap!".into());
        }
        Ok(())
    }

    fn validate_sourcemap(&self, source: &Source) -> Result<(), Error> {
        match sourcemap::decode_slice(&source.contents)? {
            sourcemap::DecodedMap::Regular(sm) => for idx in 0..sm.get_source_count() {
                let source_url = sm.get_source(idx).unwrap_or("??");
                if sm.get_source_contents(idx).is_some() || self.sources.get(source_url).is_some() {
                    info!("validator found source ({})", source_url);
                } else {
                    source.warn(format!("missing sourcecode ({})", source_url));
                }
            },
            sourcemap::DecodedMap::Index(_) => {
                source.warn("encountered indexed sourcemap. We cannot validate those.".into());
            }
        }
        Ok(())
    }

    pub fn dump_log(&self, title: &str) {
        let mut sources: Vec<_> = self.sources.values().collect();
        sources.sort_by_key(|&source| (source.ty, source.url.clone()));

        println!();
        println!("{}", style(title).dim().bold());
        let mut sect = None;

        for source in sources {
            if Some(source.ty) != sect {
                println!(
                    "  {}",
                    style(match source.ty {
                        SourceType::Script => "Scripts",
                        SourceType::MinifiedScript => "Minified Scripts",
                        SourceType::SourceMap => "Source Maps",
                    }).yellow()
                    .bold()
                );
                sect = Some(source.ty);
            }

            if source.skip_upload {
                println!("    {} [skipped separate upload]", &source.url);
            } else if source.ty == SourceType::MinifiedScript {
                let sm_ref = source.get_sourcemap_ref();
                if_chain! {
                    if sm_ref != sourcemap::SourceMapRef::Missing;
                    if let Some(url) = sm_ref.get_url();
                    then {
                        println!("    {} (sourcemap at {})",
                                 &source.url, style(url).cyan());
                    } else {
                        println!("    {} (no sourcemap ref)", &source.url);
                    }
                }
            } else {
                println!("    {}", &source.url);
            }

            if !source.messages.borrow().is_empty() {
                for msg in source.messages.borrow().iter() {
                    println!("      - {}: {}", style(&msg.0).red(), msg.1);
                }
            }
        }
    }

    /// Validates all sources within.
    pub fn validate_all(&mut self) -> Result<(), Error> {
        self.flush_pending_sources()?;
        let mut sources: Vec<_> = self.sources.iter().map(|x| x.1).collect();
        sources.sort_by_key(|x| &x.url);
        let mut failed = false;

        println!("{} Validating sources", style(">").dim());
        let pb = make_progress_bar(sources.len() as u64);
        for source in &sources {
            pb.set_message(&source.url);
            match source.ty {
                SourceType::Script | SourceType::MinifiedScript => {
                    if let Err(err) = self.validate_script(&source) {
                        source.error(format!("failed to process: {}", err));
                        failed = true;
                    }
                }
                SourceType::SourceMap => {
                    if let Err(err) = self.validate_sourcemap(&source) {
                        source.error(format!("failed to process: {}", err));
                        failed = true;
                    }
                }
            }
            pb.inc(1);
        }
        pb.finish_and_clear();

        if !failed {
            return Ok(());
        }

        self.dump_log("Source Map Validation Report");
        bail!("Encountered problems when validating source maps.");
    }

    /// Automatically rewrite all source maps.
    ///
    /// This inlines sources, flattens indexes and skips individual uploads.
    pub fn rewrite(&mut self, prefixes: &[&str]) -> Result<(), Error> {
        self.flush_pending_sources()?;

        println!("{} Rewriting sources", style(">").dim());
        let pb = make_progress_bar(self.sources.len() as u64);
        for source in self.sources.values_mut() {
            pb.set_message(&source.url);
            if source.ty != SourceType::SourceMap {
                pb.inc(1);
                continue;
            }
            let options = sourcemap::RewriteOptions {
                load_local_source_contents: true,
                strip_prefixes: prefixes,
                ..Default::default()
            };
            let sm = match sourcemap::decode_slice(&source.contents)? {
                sourcemap::DecodedMap::Regular(sm) => sm.rewrite(&options)?,
                sourcemap::DecodedMap::Index(smi) => smi.flatten_and_rewrite(&options)?,
            };
            let mut new_source: Vec<u8> = Vec::new();
            sm.to_writer(&mut new_source)?;
            source.contents = new_source;
            pb.inc(1);
        }
        pb.finish_and_clear();
        Ok(())
    }

    /// Adds sourcemap references to all minified files
    pub fn add_sourcemap_references(&mut self) -> Result<(), Error> {
        self.flush_pending_sources()?;
        let sourcemaps = HashSet::from_iter(
            self.sources
                .iter()
                .map(|x| x.1)
                .filter(|x| x.ty == SourceType::SourceMap)
                .map(|x| x.url.to_string()),
        );

        println!("{} Adding source map references", style(">").dim());
        for source in self.sources.values_mut() {
            if source.ty != SourceType::MinifiedScript {
                continue;
            }
            // we silently ignore when we can't find a sourcemap. Maybwe we should
            // log this.
            match guess_sourcemap_reference(&sourcemaps, &source.url) {
                Ok(target_url) => {
                    source.headers.push(("Sourcemap".to_string(), target_url));
                }
                Err(err) => {
                    source.messages.borrow_mut().push((
                        LogLevel::Warning,
                        format!("could not determine a source map reference ({})", err),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Uploads all files
    pub fn upload(
        &mut self,
        api: &Api,
        org: &str,
        project: Option<&str>,
        release: &str,
        dist: Option<&str>,
    ) -> Result<(), Error> {
        self.flush_pending_sources()?;

        // get a list of release files first so we know the file IDs of
        // files that already exist.
        let release_files: HashMap<_, _> = api
            .list_release_files(org, project, release)?
            .into_iter()
            .map(|artifact| ((artifact.dist, artifact.name), artifact.id))
            .collect();

        println!(
            "{} Uploading source maps for release {}",
            style(">").dim(),
            style(release).cyan()
        );

        let pb = make_progress_bar(self.sources.len() as u64);
        for source in self.sources.values() {
            pb.tick();
            if source.skip_upload {
                pb.inc(1);
                continue;
            }
            pb.set_message(&source.url);

            // try to delete old file if we have one
            if let Some(old_id) = release_files.get(&(dist.map(|x| x.into()), source.url.clone())) {
                api.delete_release_file(org, project, &release, &old_id)
                    .ok();
            }

            api.upload_release_file(
                org,
                project,
                &release,
                &FileContents::FromBytes(&source.contents),
                &source.url,
                dist,
                Some(source.headers.as_slice()),
            )?;
            pb.inc(1);
        }
        pb.finish_and_clear();

        self.dump_log("Source Map Upload Report");
        Ok(())
    }
}

impl Default for SourceMapProcessor {
    fn default() -> Self {
        SourceMapProcessor::new()
    }
}

#[test]
fn test_split_url() {
    assert_eq!(split_url("/foo.js"), (Some(""), "foo", Some("js")));
    assert_eq!(split_url("foo.js"), (None, "foo", Some("js")));
    assert_eq!(split_url("foo"), (None, "foo", None));
    assert_eq!(split_url("/foo"), (Some(""), "foo", None));
    assert_eq!(
        split_url("/foo.deadbeef0123.js"),
        (Some(""), "foo", Some("deadbeef0123.js"))
    );
}

#[test]
fn test_unsplit_url() {
    assert_eq!(&unsplit_url(Some(""), "foo", Some("js")), "/foo.js");
    assert_eq!(&unsplit_url(None, "foo", Some("js")), "foo.js");
    assert_eq!(&unsplit_url(None, "foo", None), "foo");
    assert_eq!(&unsplit_url(Some(""), "foo", None), "/foo");
}
