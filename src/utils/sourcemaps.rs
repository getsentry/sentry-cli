//! Provides sourcemap validation functionality.
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::{BufWriter, Read};
use std::iter::FromIterator;
use std::mem;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;

use console::{style, Term};
use failure::{bail, Error};
use if_chain::if_chain;
use log::{debug, info, warn};
use parking_lot::RwLock;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use symbolic::common::ByteView;
use symbolic::debuginfo::sourcebundle::{SourceBundleWriter, SourceFileInfo, SourceFileType};
use url::Url;

use crate::api::{
    Api, ChunkUploadCapability, ChunkUploadOptions, ChunkedFileState, FileContents, ProgressBarMode,
};
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::enc::decode_unknown_string;
use crate::utils::fs::{get_sha1_checksums, TempFile};
use crate::utils::progress::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Fallback concurrency for release file uploads.
static DEFAULT_CONCURRENCY: usize = 4;

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

fn url_to_bundle_path(url: &str) -> Result<String, Error> {
    let base = Url::parse("http://~").unwrap();
    let url = if url.starts_with("~/") {
        base.join(&url[2..])?
    } else {
        base.join(url)?
    };

    let mut path = url.path();
    if path.starts_with('/') {
        path = &path[1..];
    }

    Ok(match url.host_str() {
        Some("~") => format!("_/_/{}", path),
        Some(host) => format!("{}/{}/{}", url.scheme(), host, path),
        None => format!("{}/_/{}", url.scheme(), path),
    })
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

#[derive(PartialEq, Debug, Copy, Clone)]
enum LogLevel {
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    ty: SourceFileType,
    skip_upload: bool,
    headers: Vec<(String, String)>,
    messages: RwLock<Vec<(LogLevel, String)>>,
}

impl Clone for Source {
    fn clone(&self) -> Source {
        Source {
            url: self.url.clone(),
            file_path: self.file_path.clone(),
            contents: self.contents.clone(),
            ty: self.ty,
            skip_upload: self.skip_upload,
            headers: self.headers.clone(),
            messages: RwLock::new(self.messages.read().clone()),
        }
    }
}

pub struct SourceMapProcessor {
    pending_sources: HashSet<(String, PathBuf)>,
    sources: HashMap<String, Source>,
}

impl Source {
    fn log(&self, level: LogLevel, msg: String) {
        self.messages.write().push((level, msg));
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

pub struct UploadContext<'a> {
    pub org: &'a str,
    pub project: Option<&'a str>,
    pub release: &'a str,
    pub dist: Option<&'a str>,
    pub wait: bool,
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
            f.read_to_end(&mut contents)?;
            let ty = if sourcemap::is_sourcemap_slice(&contents) {
                SourceFileType::SourceMap
            } else if path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|x| x.ends_with("bundle"))
                .unwrap_or(false)
                && sourcemap::ram_bundle::is_ram_bundle_slice(&contents)
            {
                SourceFileType::IndexedRamBundle
            } else if path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|x| x.contains(".min."))
                .unwrap_or(false)
                || is_likely_minified_js(&contents)
            {
                SourceFileType::MinifiedSource
            } else {
                SourceFileType::Source
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
                    messages: RwLock::new(vec![]),
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
        } else if source.ty == SourceFileType::MinifiedSource {
            source.error("missing sourcemap!".into());
        }
        Ok(())
    }

    fn validate_sourcemap(&self, source: &Source) -> Result<(), Error> {
        match sourcemap::decode_slice(&source.contents)? {
            sourcemap::DecodedMap::Regular(sm) => {
                for idx in 0..sm.get_source_count() {
                    let source_url = sm.get_source(idx).unwrap_or("??");
                    if sm.get_source_contents(idx).is_some()
                        || self.sources.get(source_url).is_some()
                    {
                        info!("validator found source ({})", source_url);
                    } else {
                        source.warn(format!("missing sourcecode ({})", source_url));
                    }
                }
            }
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
                        SourceFileType::Source => "Scripts",
                        SourceFileType::MinifiedSource => "Minified Scripts",
                        SourceFileType::SourceMap => "Source Maps",
                        SourceFileType::IndexedRamBundle => "Indexed RAM Bundles (expanded)",
                    })
                    .yellow()
                    .bold()
                );
                sect = Some(source.ty);
            }

            if source.skip_upload {
                println!("    {} [skipped separate upload]", &source.url);
            } else if source.ty == SourceFileType::MinifiedSource {
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

            if !source.messages.read().is_empty() {
                for msg in source.messages.read().iter() {
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
                SourceFileType::Source | SourceFileType::MinifiedSource => {
                    if let Err(err) = self.validate_script(&source) {
                        source.error(format!("failed to process: {}", err));
                        failed = true;
                    }
                }
                SourceFileType::SourceMap => {
                    if let Err(err) = self.validate_sourcemap(&source) {
                        source.error(format!("failed to process: {}", err));
                        failed = true;
                    }
                }
                SourceFileType::IndexedRamBundle => (),
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

    /// Unpacks the given RAM bundle into a list of module sources and their sourcemaps
    pub fn unpack_ram_bundle(
        &mut self,
        ram_bundle: &sourcemap::ram_bundle::RamBundle,
        bundle_source_url: &str,
    ) -> Result<(), Error> {
        // We need this to flush all pending sourcemaps
        self.flush_pending_sources()?;

        debug!("Trying to guess the sourcemap reference");
        let sourcemaps_references = HashSet::from_iter(
            self.sources
                .values()
                .filter(|x| x.ty == SourceFileType::SourceMap)
                .map(|x| x.url.to_string()),
        );

        let sourcemap_url =
            match guess_sourcemap_reference(&sourcemaps_references, bundle_source_url) {
                Ok(filename) => {
                    let (path, _, _) = split_url(bundle_source_url);
                    unsplit_url(path, &filename, None)
                }
                Err(_) => {
                    warn!("Sourcemap reference for {} not found!", bundle_source_url);
                    return Ok(());
                }
            };
        debug!(
            "Sourcemap reference for {} found: {}",
            bundle_source_url, sourcemap_url
        );

        let sourcemap_content = match self.sources.get(&sourcemap_url) {
            Some(source) => &source.contents,
            None => {
                warn!(
                    "Cannot find the sourcemap for the RAM bundle using the URL: {}, skipping",
                    sourcemap_url
                );
                return Ok(());
            }
        };

        let sourcemap_index = match sourcemap::decode_slice(sourcemap_content)? {
            sourcemap::DecodedMap::Regular(_) => {
                warn!("Invalid sourcemap type for RAM bundle, skipping");
                return Ok(());
            }
            sourcemap::DecodedMap::Index(sourcemap_index) => sourcemap_index,
        };

        // We don't need the RAM bundle sourcemap itself
        self.sources.remove(&sourcemap_url);

        let ram_bundle_iter =
            sourcemap::ram_bundle::split_ram_bundle(&ram_bundle, &sourcemap_index).unwrap();
        for result in ram_bundle_iter {
            let (name, sourceview, sourcemap) = result?;

            debug!("Inserting source for {}", name);
            let source_url = join_url(&bundle_source_url, &name)?;
            self.sources.insert(
                source_url.clone(),
                Source {
                    url: source_url.clone(),
                    file_path: PathBuf::from(name.clone()),
                    contents: sourceview.source().as_bytes().to_vec(),
                    ty: SourceFileType::MinifiedSource,
                    skip_upload: false,
                    headers: vec![],
                    messages: RwLock::new(vec![]),
                },
            );

            debug!("Inserting sourcemap for {}", name);
            let sourcemap_name = format!("{}.map", name);
            let sourcemap_url = join_url(bundle_source_url, &sourcemap_name)?;
            let mut sourcemap_content: Vec<u8> = vec![];
            sourcemap.to_writer(&mut sourcemap_content)?;
            self.sources.insert(
                sourcemap_url.clone(),
                Source {
                    url: sourcemap_url.clone(),
                    file_path: PathBuf::from(sourcemap_name),
                    contents: sourcemap_content,
                    ty: SourceFileType::SourceMap,
                    skip_upload: false,
                    headers: vec![],
                    messages: RwLock::new(vec![]),
                },
            );
        }
        Ok(())
    }

    /// Replaces indexed RAM bundle entries with their expanded sources and sourcemaps
    pub fn unpack_indexed_ram_bundles(&mut self) -> Result<(), Error> {
        let mut ram_bundles = Vec::new();

        // Drain RAM bundles from self.sources
        for (url, source) in mem::replace(&mut self.sources, Default::default()).into_iter() {
            if source.ty == SourceFileType::IndexedRamBundle {
                ram_bundles.push(source);
            } else {
                self.sources.insert(url, source);
            }
        }

        for bundle_source in ram_bundles {
            debug!(
                "Parsing RAM bundle ({})...",
                bundle_source.file_path.display()
            );
            let ram_bundle = sourcemap::ram_bundle::RamBundle::parse_indexed_from_slice(
                &bundle_source.contents,
            )?;
            self.unpack_ram_bundle(&ram_bundle, &bundle_source.url)?;
        }
        Ok(())
    }

    /// Automatically rewrite all source maps.
    ///
    /// This inlines sources, flattens indexes and skips individual uploads.
    pub fn rewrite(&mut self, prefixes: &[&str]) -> Result<(), Error> {
        self.flush_pending_sources()?;

        println!("{} Rewriting sources", style(">").dim());

        self.unpack_indexed_ram_bundles()?;

        let pb = make_progress_bar(self.sources.len() as u64);
        for source in self.sources.values_mut() {
            pb.set_message(&source.url);
            if source.ty != SourceFileType::SourceMap {
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
                .filter(|x| x.ty == SourceFileType::SourceMap)
                .map(|x| x.url.to_string()),
        );

        println!("{} Adding source map references", style(">").dim());
        for source in self.sources.values_mut() {
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }
            // we silently ignore when we can't find a sourcemap. Maybe we should
            // log this.
            match guess_sourcemap_reference(&sourcemaps, &source.url) {
                Ok(target_url) => {
                    source.headers.push(("Sourcemap".to_string(), target_url));
                }
                Err(err) => {
                    source.messages.write().push((
                        LogLevel::Warning,
                        format!("could not determine a source map reference ({})", err),
                    ));
                }
            }
        }
        Ok(())
    }

    fn upload_files_parallel(
        &self,
        context: &UploadContext<'_>,
        num_threads: usize,
    ) -> Result<(), Error> {
        let api = Api::current();

        // get a list of release files first so we know the file IDs of
        // files that already exist.
        let release_files: HashMap<_, _> = api
            .list_release_files(context.org, context.project, context.release)?
            .into_iter()
            .map(|artifact| ((artifact.dist, artifact.name), artifact.id))
            .collect();

        println!(
            "{} Uploading source maps for release {}",
            style(">").dim(),
            style(context.release).cyan()
        );

        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} Uploading {} source map{}...\
             \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
            style(">").dim(),
            style(self.sources.len().to_string()).yellow(),
            if self.sources.len() == 1 { "" } else { "s" }
        ));

        let sources = self
            .sources
            .values()
            .filter(|source| !source.skip_upload)
            .collect::<Vec<_>>();

        let total_bytes = sources
            .iter()
            .map(|source| source.contents.len() as u64)
            .sum();

        let pb = Arc::new(ProgressBar::new(total_bytes));
        pb.set_style(progress_style);

        let pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
        let bytes = Arc::new(RwLock::new(vec![0u64; sources.len()]));

        pool.install(|| {
            sources
                .into_par_iter()
                .enumerate()
                .map(|(index, source)| -> Result<(), Error> {
                    let api = Api::current();
                    let mode = ProgressBarMode::Shared((
                        pb.clone(),
                        source.contents.len() as u64,
                        index,
                        bytes.clone(),
                    ));

                    if let Some(old_id) =
                        release_files.get(&(context.dist.map(|x| x.into()), source.url.clone()))
                    {
                        api.delete_release_file(
                            context.org,
                            context.project,
                            &context.release,
                            &old_id,
                        )
                        .ok();
                    }

                    api.upload_release_file(
                        context.org,
                        context.project,
                        context.release,
                        &FileContents::FromBytes(&source.contents),
                        &source.url,
                        context.dist,
                        Some(source.headers.as_slice()),
                        mode,
                    )?;

                    Ok(())
                })
                .collect::<Result<(), _>>()
        })?;

        pb.finish_and_clear();

        Ok(())
    }

    fn build_artifact_bundle(&self, context: &UploadContext<'_>) -> Result<TempFile, Error> {
        let sources = self
            .sources
            .values()
            .filter(|source| !source.skip_upload)
            .collect::<Vec<_>>();

        let progress_style = ProgressStyle::default_bar().template(
            "{prefix:.dim} Bundling files for upload... {msg:.dim}\
             \n{wide_bar}  {pos}/{len}",
        );

        let progress = ProgressBar::new(sources.len() as u64);
        progress.set_style(progress_style);
        progress.set_prefix(">");

        let archive = TempFile::create()?;
        let mut bundle = SourceBundleWriter::start(BufWriter::new(archive.open()?))?;

        bundle.set_attribute("org".to_owned(), context.org.to_owned());
        if let Some(project) = context.project {
            bundle.set_attribute("project".to_owned(), project.to_owned());
        }
        bundle.set_attribute("release".to_owned(), context.release.to_owned());
        if let Some(dist) = context.dist {
            bundle.set_attribute("dist".to_owned(), dist.to_owned());
        }

        for source in self.sources.values() {
            progress.inc(1);
            progress.set_message(&source.url);

            let mut info = SourceFileInfo::new();
            info.set_ty(source.ty);
            info.set_url(source.url.clone());
            for (k, v) in &source.headers {
                info.add_header(k.clone(), v.clone());
            }

            let bundle_path = url_to_bundle_path(&source.url)?;
            bundle.add_file(bundle_path, source.contents.as_slice(), info)?;
        }

        bundle.finish()?;

        progress.finish_and_clear();
        println!(
            "{} Bundled {} {} for upload",
            style(">").dim(),
            style(sources.len()).yellow(),
            match sources.len() {
                1 => "file",
                _ => "files",
            }
        );

        Ok(archive)
    }

    fn upload_files_chunked(
        &self,
        context: &UploadContext<'_>,
        options: &ChunkUploadOptions,
    ) -> Result<(), Error> {
        let archive = self.build_artifact_bundle(&context)?;

        let progress_style =
            ProgressStyle::default_spinner().template("{spinner} Optimizing bundle for upload...");

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(100);
        progress.set_style(progress_style);

        let view = ByteView::open(archive.path())?;
        let (checksum, checksums) = get_sha1_checksums(&view, options.chunk_size)?;
        let chunks = view
            .chunks(options.chunk_size as usize)
            .zip(checksums.iter())
            .map(|(data, checksum)| Chunk((*checksum, data)))
            .collect::<Vec<_>>();

        progress.finish_and_clear();

        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} Uploading release files...\
             \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
            style(">").dim(),
        ));

        upload_chunks(&chunks, options, progress_style)?;
        println!("{} Uploaded release files to Sentry", style(">").dim(),);

        let progress_style =
            ProgressStyle::default_spinner().template("{spinner} Processing files...");

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(100);
        progress.set_style(progress_style);

        let api = Api::current();
        let response = loop {
            let response =
                api.assemble_artifacts(context.org, context.release, checksum, &checksums)?;

            // Poll until there is a response, unless the user has specified to skip polling. In
            // that case, we return the potentially partial response from the server. This might
            // still contain a cached error.
            if !context.wait || response.state.finished() {
                break response;
            }

            std::thread::sleep(ASSEMBLE_POLL_INTERVAL);
        };

        if response.state == ChunkedFileState::Error {
            let message = match response.detail {
                Some(ref detail) => detail,
                None => "unknown error",
            };

            bail!("Failed to process uploaded files: {}", message);
        }

        progress.finish_and_clear();
        println!("{} File processing complete", style(">").dim());

        Ok(())
    }

    fn do_upload(&self, context: &UploadContext<'_>) -> Result<(), Error> {
        let api = Api::current();

        let chunk_options = api.get_chunk_upload_options(context.org)?;
        if let Some(ref chunk_options) = chunk_options {
            if chunk_options.supports(ChunkUploadCapability::ReleaseFiles) {
                return self.upload_files_chunked(context, chunk_options);
            }
        }

        // Do not permit uploads of more than 20k files if the server does not
        // support artifact bundles.  This is a termporary downside protection to
        // protect users from uploading more sources than we support.
        if self.sources.len() > 20_000 {
            bail!(
                "Too many sources: {} exceeds maximum allowed files per release",
                self.sources.len()
            );
        }

        let concurrency = chunk_options.map_or(DEFAULT_CONCURRENCY, |o| usize::from(o.concurrency));
        self.upload_files_parallel(context, concurrency)
    }

    /// Uploads all files
    pub fn upload(&mut self, context: &UploadContext<'_>) -> Result<(), Error> {
        self.flush_pending_sources()?;

        self.do_upload(context)?;
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
    assert_eq!(
        split_url("/foo/bar/baz.js"),
        (Some("/foo/bar"), "baz", Some("js"))
    );
}

#[test]
fn test_unsplit_url() {
    assert_eq!(&unsplit_url(Some(""), "foo", Some("js")), "/foo.js");
    assert_eq!(&unsplit_url(None, "foo", Some("js")), "foo.js");
    assert_eq!(&unsplit_url(None, "foo", None), "foo");
    assert_eq!(&unsplit_url(Some(""), "foo", None), "/foo");
}
