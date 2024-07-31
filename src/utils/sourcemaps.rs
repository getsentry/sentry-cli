//! Provides sourcemap validation functionality.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::io::Write;
use std::mem;
use std::path::{Path, PathBuf};
use std::str;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Error, Result};
use console::style;
use indicatif::ProgressStyle;
use log::{debug, info, warn};
use sentry::types::DebugId;
use sha1_smol::Digest;
use sourcemap::SourceMap;
use symbolic::debuginfo::js::{
    discover_debug_id, discover_sourcemap_embedded_debug_id, discover_sourcemaps_location,
};
use symbolic::debuginfo::sourcebundle::SourceFileType;
use url::Url;

use crate::api::Api;
use crate::utils::file_search::ReleaseFileMatch;
use crate::utils::file_upload::{
    initialize_legacy_release_upload, FileUpload, SourceFile, SourceFiles, UploadContext,
};
use crate::utils::logging::is_quiet_mode;
use crate::utils::progress::ProgressBar;
use crate::utils::sourcemaps::inject::InjectReport;

pub mod inject;

/// The string prefix denoting a data URL.
///
/// Data URLs are used to embed sourcemaps directly in javascript source files.
const DATA_PREAMBLE: &str = "data:application/json;base64,";

fn join_url(base_url: &str, url: &str) -> Result<String> {
    if base_url.starts_with("~/") {
        match Url::parse(&format!("http://{base_url}"))?.join(url) {
            Ok(url) => {
                let rv = url.to_string();
                if let Some(rest) = rv.strip_prefix("http://~/") {
                    Ok(format!("~/{rest}"))
                } else {
                    Ok(rv)
                }
            }
            Err(x) => Err(Error::from(x).context("could not join URL")),
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

pub fn get_sourcemap_ref_from_headers(file: &SourceFile) -> Option<sourcemap::SourceMapRef> {
    get_sourcemap_reference_from_headers(file.headers.iter())
        .map(|sm_ref| sourcemap::SourceMapRef::Ref(sm_ref.to_string()))
}

pub fn get_sourcemap_ref_from_contents(file: &SourceFile) -> Option<sourcemap::SourceMapRef> {
    sourcemap::locate_sourcemap_reference_slice(&file.contents).unwrap_or(None)
}

pub fn get_sourcemap_ref(file: &SourceFile) -> Option<sourcemap::SourceMapRef> {
    get_sourcemap_ref_from_headers(file).or_else(|| get_sourcemap_ref_from_contents(file))
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

fn guess_sourcemap_reference(
    sourcemaps: &HashSet<String>,
    min_url: &str,
) -> Result<SourceMapReference> {
    // if there is only one sourcemap in total we just assume that's the one.
    // We just need to make sure that we fix up the reference if we need to
    // (eg: ~/ -> /).
    if sourcemaps.len() == 1 {
        let original_url = sourcemaps.iter().next().unwrap();
        return Ok(SourceMapReference {
            url: sourcemap::make_relative_path(min_url, original_url),
            original_url: Option::from(original_url.to_string()),
        });
    }

    let map_ext = "map";
    let (path, basename, ext) = split_url(min_url);

    // foo.min.js -> foo.map
    if sourcemaps.contains(&unsplit_url(path, basename, Some("map"))) {
        return Ok(SourceMapReference::from_url(unsplit_url(
            None,
            basename,
            Some("map"),
        )));
    }

    if let Some(ext) = ext.as_ref() {
        // foo.min.js -> foo.min.js.map
        let new_ext = format!("{ext}.{map_ext}");
        if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
            return Ok(SourceMapReference::from_url(unsplit_url(
                None,
                basename,
                Some(&new_ext),
            )));
        }

        // foo.min.js -> foo.js.map
        if let Some(rest) = ext.strip_prefix("min.") {
            let new_ext = format!("{rest}.{map_ext}");
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(SourceMapReference::from_url(unsplit_url(
                    None,
                    basename,
                    Some(&new_ext),
                )));
            }
        }

        // foo.min.js -> foo.min.map
        let mut parts: Vec<_> = ext.split('.').collect();
        if parts.len() > 1 {
            let parts_len = parts.len();
            parts[parts_len - 1] = map_ext;
            let new_ext = parts.join(".");
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(SourceMapReference::from_url(unsplit_url(
                    None,
                    basename,
                    Some(&new_ext),
                )));
            }
        }
    }

    bail!("Could not auto-detect referenced sourcemap for {}", min_url);
}

/// Container to cary relative computed source map url.
/// and original url with which the file was added to the processor.
/// This enable us to look up the source map file based on the original url.
/// Which can be used for example for debug id referencing.
pub struct SourceMapReference {
    url: String,
    original_url: Option<String>,
}

impl SourceMapReference {
    pub fn from_url(url: String) -> Self {
        SourceMapReference {
            url,
            original_url: None,
        }
    }
}

pub struct SourceMapProcessor {
    pending_sources: HashSet<(String, ReleaseFileMatch)>,
    sources: SourceFiles,
    sourcemap_references: HashMap<String, Option<SourceMapReference>>,
    debug_ids: HashMap<String, DebugId>,
}

fn is_hermes_bytecode(slice: &[u8]) -> bool {
    // The hermes bytecode format magic is defined here:
    // https://github.com/facebook/hermes/blob/5243222ef1d92b7393d00599fc5cff01d189a88a/include/hermes/BCGen/HBC/BytecodeFileFormat.h#L24-L25
    const HERMES_MAGIC: [u8; 8] = [0xC6, 0x1F, 0xBC, 0x03, 0xC1, 0x03, 0x19, 0x1F];
    slice.starts_with(&HERMES_MAGIC)
}

fn url_matches_extension(url: &str, extensions: &[&str]) -> bool {
    if extensions.is_empty() {
        return true;
    }

    match url.rsplit('/').next() {
        Some(filename) => extensions
            .iter()
            .any(|ext| filename.ends_with(&format!(".{ext}"))),
        None => false,
    }
}

/// Return true iff url is a remote url (not a local path or embedded sourcemap).
fn is_remote_url(url: &str) -> bool {
    return match Url::parse(url) {
        Ok(url) => url.scheme() != "data",
        Err(_) => false,
    };
}

/// Return true if url appears to be a URL path.
/// Most often, a URL path will begin with `/`,
/// particularly in the case of static asset collection and hosting,
/// but such a path is very unlikely to exist in the local filesystem.
fn is_url_path(url: &str) -> bool {
    url.starts_with('/') && !Path::new(url).exists()
}

/// Return true iff url is probably not a local file path.
fn is_remote_sourcemap(url: &str) -> bool {
    is_remote_url(url) || is_url_path(url)
}

impl SourceMapProcessor {
    /// Creates a new sourcemap validator.
    pub fn new() -> SourceMapProcessor {
        SourceMapProcessor {
            pending_sources: HashSet::new(),
            sources: SourceFiles::new(),
            sourcemap_references: HashMap::new(),
            debug_ids: HashMap::new(),
        }
    }

    /// Adds a new file for processing.
    pub fn add(&mut self, url: &str, file: ReleaseFileMatch) -> Result<()> {
        self.pending_sources.insert((url.to_string(), file));
        Ok(())
    }

    fn flush_pending_sources(&mut self) {
        if self.pending_sources.is_empty() {
            return;
        }

        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} {{msg}}\n{{wide_bar}} {{pos}}/{{len}}",
            style(">").cyan()
        ));
        let pb = ProgressBar::new(self.pending_sources.len());
        pb.set_style(progress_style);

        println!(
            "{} Analyzing {} sources",
            style(">").dim(),
            style(self.pending_sources.len()).yellow()
        );
        for (url, mut file) in self.pending_sources.drain() {
            pb.set_message(&url);

            let (ty, debug_id) = if sourcemap::is_sourcemap_slice(&file.contents) {
                (
                    SourceFileType::SourceMap,
                    std::str::from_utf8(&file.contents)
                        .ok()
                        .and_then(discover_sourcemap_embedded_debug_id),
                )
            } else if file
                .path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|x| x.ends_with("bundle"))
                .unwrap_or(false)
                && sourcemap::ram_bundle::is_ram_bundle_slice(&file.contents)
            {
                (SourceFileType::IndexedRamBundle, None)
            } else if is_hermes_bytecode(&file.contents) {
                // This is actually a big hack:
                // For the react-native Hermes case, we skip uploading the bytecode bundle,
                // and rather flag it as an empty "minified source". That way, it
                // will get a SourceMap reference, and the server side processor
                // should deal with it accordingly.
                file.contents.clear();
                (SourceFileType::MinifiedSource, None)
            } else {
                // Here, we use MinifiedSource for historical reasons. We used to guess whether
                // a JS file was a minified file or a source file, and we would treat these files
                // differently when uploading or injecting them. However, the desired behavior is
                // and has always been to treat all JS files the same, since users should be
                // responsible for providing the file paths for only files they would like to have
                // uploaded or injected. The minified file guessing furthermore was not reliable,
                // since minification is not a necessary step in the JS build process.
                //
                // We use MinifiedSource here rather than Source because we want to treat all JS
                // files the way we used to treat minified files only. To use Source, we would need
                // to analyze all possible code paths that check this value, and update those as
                // well. To keep the change minimal, we use MinifiedSource here.
                (
                    SourceFileType::MinifiedSource,
                    std::str::from_utf8(&file.contents)
                        .ok()
                        .and_then(discover_debug_id),
                )
            };

            let mut source_file = SourceFile {
                url: url.clone(),
                path: file.path,
                contents: file.contents,
                ty,
                headers: BTreeMap::new(),
                messages: vec![],
                already_uploaded: false,
            };

            if let Some(debug_id) = debug_id {
                source_file.set_debug_id(debug_id.to_string());
                self.debug_ids.insert(url.clone(), debug_id);
            }

            self.sources.insert(url.clone(), source_file);
            pb.inc(1);
        }
        pb.finish_with_duration("Analyzing");
    }

    /// Collect references to sourcemaps in minified source files
    /// and saves them in `self.sourcemap_references`.
    fn collect_sourcemap_references(&mut self) {
        let sourcemaps = self
            .sources
            .iter()
            .map(|x| x.1)
            .filter(|x| x.ty == SourceFileType::SourceMap)
            .map(|x| x.url.to_string())
            .collect();

        for source in self.sources.values_mut() {
            // Skip everything but minified JS files.
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }

            if self.sourcemap_references.contains_key(&source.url) {
                continue;
            }

            let Ok(contents) = std::str::from_utf8(&source.contents) else {
                continue;
            };

            // If this is a full external URL, the code below is going to attempt
            // to "normalize" it with the source path, resulting in a bogus path
            // like "path/to/source/dir/https://some-static-host.example.com/path/to/foo.js.map"
            // that can't be resolved to a source map file.
            // Instead, we pretend we failed to discover the location, and we fall back to
            // guessing the source map location based on the source location.
            let location =
                discover_sourcemaps_location(contents).filter(|loc| !is_remote_sourcemap(loc));
            let sourcemap_reference = match location {
                Some(url) => SourceMapReference::from_url(url.to_string()),
                None => match guess_sourcemap_reference(&sourcemaps, &source.url) {
                    Ok(target) => target,
                    Err(err) => {
                        source.warn(format!(
                            "could not determine a source map reference ({err})"
                        ));
                        self.sourcemap_references
                            .insert(source.url.to_string(), None);
                        continue;
                    }
                },
            };

            self.sourcemap_references
                .insert(source.url.to_string(), Some(sourcemap_reference));
        }
    }

    pub fn dump_log(&self, title: &str) {
        if is_quiet_mode() {
            return;
        }

        let mut sources: Vec<_> = self.sources.values().collect();
        sources.sort_by_key(|&source| (source.ty, source.url.clone()));

        println!();
        println!("{}", style(title).dim().bold());
        let mut current_section = None;

        for source in sources {
            let section_title = match source.ty {
                SourceFileType::Source | SourceFileType::MinifiedSource => "Scripts",
                SourceFileType::SourceMap => "Source Maps",
                SourceFileType::IndexedRamBundle => "Indexed RAM Bundles (expanded)",
            };

            if Some(section_title) != current_section {
                println!("  {}", style(section_title).yellow().bold());
                current_section = Some(section_title);
            }

            if source.already_uploaded {
                println!(
                    "    {}",
                    style(format!("{} (skipped; already uploaded)", &source.url)).yellow()
                );
                continue;
            }

            let mut pieces = Vec::new();

            if [SourceFileType::Source, SourceFileType::MinifiedSource].contains(&source.ty) {
                if let Some(sm_ref) = get_sourcemap_ref(source) {
                    let sm_url = sm_ref.get_url();
                    if sm_url.starts_with("data:") {
                        pieces.push("embedded sourcemap".to_string());
                    } else {
                        pieces.push(format!("sourcemap at {}", style(sm_url).cyan()));
                    };
                } else {
                    pieces.push("no sourcemap ref".into());
                }
            }

            if let Some(debug_id) = source.debug_id() {
                pieces.push(format!("debug id {}", style(debug_id).yellow()));
            }

            if pieces.is_empty() {
                println!("    {}", source.url);
            } else {
                println!("    {} ({})", source.url, pieces.join(", "));
            }

            for msg in source.messages.iter() {
                println!("      - {}: {}", style(&msg.0).red(), msg.1);
            }
        }
    }

    /// Validates all sources within.
    pub fn validate_all(&mut self) -> Result<()> {
        self.flush_pending_sources();
        let source_urls = self.sources.keys().cloned().collect();
        let sources: Vec<&mut SourceFile> = self.sources.values_mut().collect();
        let mut failed = false;

        println!("{} Validating sources", style(">").dim());

        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} {{msg}}\n{{wide_bar}} {{pos}}/{{len}}",
            style(">").cyan()
        ));
        let pb = ProgressBar::new(sources.len());
        pb.set_style(progress_style);

        for source in sources {
            pb.set_message(&source.url);
            match source.ty {
                SourceFileType::Source | SourceFileType::MinifiedSource => {
                    if let Err(err) = validate_script(source) {
                        source.error(format!("failed to process: {err}"));
                        failed = true;
                    }
                }
                SourceFileType::SourceMap => {
                    if let Err(err) = validate_sourcemap(&source_urls, source) {
                        source.error(format!("failed to process: {err}"));
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
    ) -> Result<()> {
        // We need this to flush all pending sourcemaps
        self.flush_pending_sources();

        debug!("Trying to guess the sourcemap reference");
        let sourcemaps_references = self
            .sources
            .values()
            .filter(|x| x.ty == SourceFileType::SourceMap)
            .map(|x| x.url.to_string())
            .collect();

        let sourcemap_url =
            match guess_sourcemap_reference(&sourcemaps_references, bundle_source_url) {
                Ok(filename) => {
                    let (path, _, _) = split_url(bundle_source_url);
                    unsplit_url(path, &filename.url, None)
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

        let Some(sourcemap_source) = self.sources.get(&sourcemap_url) else {
            warn!(
                "Cannot find the sourcemap for the RAM bundle using the URL: {}, skipping",
                sourcemap_url
            );
            return Ok(());
        };
        let sourcemap_content = &sourcemap_source.contents;

        let sourcemap_index = match sourcemap::decode_slice(sourcemap_content)? {
            sourcemap::DecodedMap::Index(sourcemap_index) => sourcemap_index,
            _ => {
                warn!("Invalid sourcemap type for RAM bundle, skipping");
                return Ok(());
            }
        };

        // We have to include the bundle sourcemap which is the first section
        // in the bundle source map before the modules maps
        if let Some(index_section) = &sourcemap_index
            .sections()
            .nth(0)
            .and_then(|index_section| index_section.get_sourcemap())
        {
            let mut index_sourcemap_content: Vec<u8> = vec![];
            index_section.to_writer(&mut index_sourcemap_content)?;
            self.sources.insert(
                sourcemap_url.clone(),
                SourceFile {
                    url: sourcemap_source.url.clone(),
                    path: sourcemap_source.path.clone(),
                    contents: index_sourcemap_content,
                    ty: SourceFileType::SourceMap,
                    headers: sourcemap_source.headers.clone(),
                    messages: sourcemap_source.messages.clone(),
                    already_uploaded: false,
                },
            );
        }

        let ram_bundle_iter =
            sourcemap::ram_bundle::split_ram_bundle(ram_bundle, &sourcemap_index).unwrap();
        for result in ram_bundle_iter {
            let (name, sourceview, sourcemap) = result?;

            debug!("Inserting source for {}", name);
            let source_url = join_url(bundle_source_url, &name)?;
            self.sources.insert(
                source_url.clone(),
                SourceFile {
                    url: source_url.clone(),
                    path: PathBuf::from(name.clone()),
                    contents: sourceview.source().as_bytes().to_vec(),
                    ty: SourceFileType::MinifiedSource,
                    headers: BTreeMap::new(),
                    messages: vec![],
                    already_uploaded: false,
                },
            );

            debug!("Inserting sourcemap for {}", name);
            let sourcemap_name = format!("{name}.map");
            let sourcemap_url = join_url(bundle_source_url, &sourcemap_name)?;
            let mut sourcemap_content: Vec<u8> = vec![];
            sourcemap.to_writer(&mut sourcemap_content)?;
            self.sources.insert(
                sourcemap_url.clone(),
                SourceFile {
                    url: sourcemap_url.clone(),
                    path: PathBuf::from(sourcemap_name),
                    contents: sourcemap_content,
                    ty: SourceFileType::SourceMap,
                    headers: BTreeMap::new(),
                    messages: vec![],
                    already_uploaded: false,
                },
            );
        }
        Ok(())
    }

    /// Replaces indexed RAM bundle entries with their expanded sources and sourcemaps
    pub fn unpack_indexed_ram_bundles(&mut self) -> Result<()> {
        let mut ram_bundles = Vec::new();

        // Drain RAM bundles from self.sources
        for (url, source) in mem::take(&mut self.sources).into_iter() {
            if source.ty == SourceFileType::IndexedRamBundle {
                ram_bundles.push(source);
            } else {
                self.sources.insert(url, source);
            }
        }

        for bundle_source in ram_bundles {
            debug!("Parsing RAM bundle ({})...", bundle_source.path.display());
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
    pub fn rewrite(&mut self, prefixes: &[&str]) -> Result<()> {
        self.flush_pending_sources();

        println!("{} Rewriting sources", style(">").dim());

        self.unpack_indexed_ram_bundles()?;

        let progress_style = ProgressStyle::default_bar().template(&format!(
            "{} {{msg}}\n{{wide_bar}} {{pos}}/{{len}}",
            style(">").cyan()
        ));
        let pb = ProgressBar::new(self.sources.len());
        pb.set_style(progress_style);

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
            let mut new_source: Vec<u8> = Vec::new();
            match sourcemap::decode_slice(&source.contents)? {
                sourcemap::DecodedMap::Regular(sm) => {
                    sm.rewrite(&options)?.to_writer(&mut new_source)?
                }
                sourcemap::DecodedMap::Hermes(smh) => {
                    smh.rewrite(&options)?.to_writer(&mut new_source)?
                }
                sourcemap::DecodedMap::Index(smi) => smi
                    .flatten_and_rewrite(&options)?
                    .to_writer(&mut new_source)?,
            };
            source.contents = new_source;
            pb.inc(1);
        }
        pb.finish_with_duration("Rewriting");
        Ok(())
    }

    /// Adds sourcemap references to all minified files
    pub fn add_sourcemap_references(&mut self) -> Result<()> {
        self.flush_pending_sources();
        self.collect_sourcemap_references();

        println!("{} Adding source map references", style(">").dim());
        for source in self.sources.values_mut() {
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }

            if let Some(Some(sourcemap)) = self.sourcemap_references.get(&source.url) {
                source.set_sourcemap_reference(sourcemap.url.to_string());
            }
        }
        Ok(())
    }

    /// Adds debug id to the source file headers from the linked source map.
    /// This is used for files we can't read debug ids from (e.g. Hermes bytecode bundles).
    pub fn add_debug_id_references(&mut self) -> Result<()> {
        self.flush_pending_sources();

        for source in self.sources.values_mut() {
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }

            if let Some(Some(sourcemap_reference)) = self.sourcemap_references.get(&source.url) {
                let sourcemap_url = &sourcemap_reference
                    .original_url
                    .clone()
                    .unwrap_or(sourcemap_reference.url.clone());

                if !self.debug_ids.contains_key(sourcemap_url) {
                    debug!(
                        "{} No debug id found for {} to reference",
                        style(">").dim(),
                        sourcemap_url
                    );
                    continue;
                }

                if source.debug_id().is_some() {
                    debug!(
                        "{} {} already has a debug id reference",
                        style(">").dim(),
                        source.url
                    );
                    continue;
                }

                if self.debug_ids.contains_key(&source.url) {
                    debug!("{} {} already has a debug id", style(">").dim(), source.url);
                    continue;
                }

                debug!(
                    "{} Adding debug id {} reference to {}",
                    style(">").dim(),
                    self.debug_ids[sourcemap_url].to_string(),
                    source.url
                );
                source.set_debug_id(self.debug_ids[sourcemap_url].to_string());
                self.debug_ids
                    .insert(source.url.clone(), self.debug_ids[sourcemap_url]);
            } else {
                debug!(
                    "{} No sourcemap reference found for {}",
                    style(">").dim(),
                    source.url
                );
            }
        }
        Ok(())
    }

    /// Flags the collected sources whether they have already been uploaded before
    /// (based on their checksum), and returns the number of files that *do* need an upload.
    fn flag_uploaded_sources(&mut self, context: &UploadContext<'_>) -> usize {
        let mut files_needing_upload = self.sources.len();

        // TODO: this endpoint does not exist for non release based uploads
        if !context.dedupe {
            return files_needing_upload;
        }
        let release = match context.release {
            Some(release) => release,
            None => return files_needing_upload,
        };

        let mut sources_checksums: Vec<_> = self
            .sources
            .values()
            .filter_map(|s| s.checksum().map(|c| c.to_string()).ok())
            .collect();

        // Checksums need to be sorted in order to satisfy integration tests constraints.
        sources_checksums.sort();

        let api = Api::current();

        if let Ok(artifacts) = api.authenticated().and_then(|api| {
            api.list_release_files_by_checksum(
                context.org,
                context.project,
                release,
                &sources_checksums,
            )
        }) {
            let already_uploaded_checksums: HashSet<_> = artifacts
                .into_iter()
                .filter_map(|artifact| Digest::from_str(&artifact.sha1).ok())
                .collect();

            for source in self.sources.values_mut() {
                if let Ok(checksum) = source.checksum() {
                    if already_uploaded_checksums.contains(&checksum) {
                        source.already_uploaded = true;
                        files_needing_upload -= 1;
                    }
                }
            }
        }
        files_needing_upload
    }

    /// Uploads all files, and on success, returns the number of files that were
    /// uploaded, wrapped in Ok()
    pub fn upload(&mut self, context: &UploadContext<'_>) -> Result<usize> {
        initialize_legacy_release_upload(context)?;
        self.flush_pending_sources();

        // If there is no release, we have to check that the files at least
        // contain debug ids.
        if context.release.is_none() {
            let mut files_without_debug_id = BTreeSet::new();
            let mut files_with_debug_id = false;

            for (source_url, sourcemap_url) in &self.sourcemap_references {
                if sourcemap_url.is_none() {
                    continue;
                }

                if self.debug_ids.contains_key(source_url) {
                    files_with_debug_id = true;
                } else {
                    files_without_debug_id.insert(source_url.clone());
                }
            }

            // No debug ids on any files -> can't upload
            if !files_without_debug_id.is_empty() && !files_with_debug_id {
                bail!("Cannot upload: You must either specify a release or have debug ids injected into your sources");
            }

            // At least some files don't have debug ids -> print a warning
            if !files_without_debug_id.is_empty() {
                warn!("Some source files don't have debug ids:");

                for file in files_without_debug_id {
                    warn!("- {file}");
                }
            }
        }

        let files_needing_upload = self.flag_uploaded_sources(context);
        if files_needing_upload > 0 {
            let mut uploader = FileUpload::new(context);
            uploader.files(&self.sources);
            uploader.upload()?;
            self.dump_log("Source Map Upload Report");
        } else {
            println!("{} Nothing to upload", style(">").dim());
        }
        Ok(files_needing_upload)
    }

    /// Upload all files in "strict" mode. Strict mode differs from a normal upload
    /// only when there are no files to upload. In strict mode, having no files to
    /// upload results in an error, whereas such an upload is successful in normal
    /// (non-strict) mode. On success, the number of uploaded files is returned in
    /// an Ok()
    pub fn upload_strict(&mut self, context: &UploadContext<'_>) -> Result<usize> {
        match self.upload(context) {
            Ok(0) => Err(anyhow!("No files to upload (strict mode).")),
            other => other,
        }
    }

    /// Injects debug ids into minified source files and sourcemaps.
    ///
    /// This iterates over contained minified source files and adds debug ids
    /// to them. Files already containing debug ids will be untouched.
    ///
    /// If a source file refers to a sourcemap and that sourcemap is locally
    /// available, the debug id will be injected there as well so as to tie
    /// them together. If for whatever reason the sourcemap already contains
    /// a debug id, it will be reused for the source file.
    ///
    /// If `dry_run` is false, this will modify the source and sourcemap files on disk!
    ///
    /// The `js_extensions` is a list of file extensions that should be considered
    /// for JavaScript files.
    pub fn inject_debug_ids(&mut self, dry_run: bool, js_extensions: &[&str]) -> Result<()> {
        self.flush_pending_sources();
        self.collect_sourcemap_references();
        println!("{} Injecting debug ids", style(">").dim());

        let mut report = InjectReport::default();

        let mut sourcemaps = self
            .sources
            .values()
            .filter_map(|s| (s.ty == SourceFileType::SourceMap).then_some(s.url.clone()))
            .collect::<Vec<_>>();
        sourcemaps.sort();

        for (source_url, sourcemap_url) in self.sourcemap_references.iter_mut() {
            // We only allow injection into files that match the extension
            if !url_matches_extension(source_url, js_extensions) {
                debug!(
                    "skipping potential js file {} because it does not match extension",
                    source_url
                );
                continue;
            }

            if let Some(debug_id) = self.debug_ids.get(source_url) {
                report
                    .previously_injected
                    .push((source_url.into(), *debug_id));
                continue;
            }

            // Modify the source file and the sourcemap.
            // There are several cases to consider according to whether we have a sourcemap for the source file and
            // whether it's embedded or external.
            let debug_id = match sourcemap_url {
                None => {
                    // Case 1: We have no sourcemap for the source file. Hash the file contents for the debug id.
                    let source_file = self.sources.get_mut(source_url).unwrap();
                    let debug_id = inject::debug_id_from_bytes_hashed(&source_file.contents);

                    // If we don't have a sourcemap, it's not safe to inject the code snippet at the beginning,
                    // because that would throw off all the mappings. Instead, inject the snippet at the very end.
                    // This isn't ideal, but it's the best we can do in this case.
                    inject::fixup_js_file_end(&mut source_file.contents, debug_id)
                        .context(format!("Failed to process {}", source_file.path.display()))?;
                    debug_id
                }
                Some(sourcemap) => {
                    if let Some(encoded) = sourcemap.url.strip_prefix(DATA_PREAMBLE) {
                        // Case 2: The source file has an embedded sourcemap.

                        let Ok(mut decoded) = data_encoding::BASE64.decode(encoded.as_bytes())
                        else {
                            bail!("Invalid embedded sourcemap in source file {source_url}");
                        };

                        let mut sourcemap = SourceMap::from_slice(&decoded).with_context(|| {
                            format!("Invalid embedded sourcemap in source file {source_url}")
                        })?;

                        let debug_id = sourcemap
                            .get_debug_id()
                            .unwrap_or_else(|| inject::debug_id_from_bytes_hashed(&decoded));

                        let source_file = self.sources.get_mut(source_url).unwrap();
                        let adjustment_map =
                            inject::fixup_js_file(&mut source_file.contents, debug_id).context(
                                format!("Failed to process {}", source_file.path.display()),
                            )?;

                        sourcemap.adjust_mappings(&adjustment_map);
                        sourcemap.set_debug_id(Some(debug_id));

                        decoded.clear();
                        sourcemap.to_writer(&mut decoded)?;

                        let encoded = data_encoding::BASE64.encode(&decoded);
                        let new_sourcemap_url = format!("{DATA_PREAMBLE}{encoded}");

                        inject::replace_sourcemap_url(
                            &mut source_file.contents,
                            &new_sourcemap_url,
                        )?;
                        *sourcemap_url = Some(SourceMapReference::from_url(new_sourcemap_url));

                        debug_id
                    } else {
                        // Handle external sourcemaps

                        let normalized =
                            inject::normalize_sourcemap_url(source_url, &sourcemap.url);
                        let matches = inject::find_matching_paths(&sourcemaps, &normalized);

                        let sourcemap_url = match &matches[..] {
                            [] => normalized,
                            [x] => x.to_string(),
                            _ => {
                                warn!("Ambiguous matches for sourcemap path {normalized}:");
                                for path in matches {
                                    warn!("{path}");
                                }
                                normalized
                            }
                        };

                        if self.sources.contains_key(&sourcemap_url) {
                            // Case 3: We have an external sourcemap for the source file.

                            // We need to do a bit of a dance here because we can't mutably
                            // borrow the source file and the sourcemap at the same time.
                            let (mut sourcemap, debug_id, debug_id_fresh) = {
                                let sourcemap_file = &self.sources[&sourcemap_url];

                                let sm = SourceMap::from_slice(&sourcemap_file.contents).context(
                                    format!("Invalid sourcemap at {}", sourcemap_file.url),
                                )?;

                                match sm.get_debug_id() {
                                    Some(debug_id) => (sm, debug_id, false),
                                    None => {
                                        let debug_id = inject::debug_id_from_bytes_hashed(
                                            &sourcemap_file.contents,
                                        );
                                        (sm, debug_id, true)
                                    }
                                }
                            };

                            let source_file = self.sources.get_mut(source_url).unwrap();
                            let adjustment_map =
                                inject::fixup_js_file(&mut source_file.contents, debug_id)
                                    .context(format!(
                                        "Failed to process {}",
                                        source_file.path.display()
                                    ))?;

                            sourcemap.adjust_mappings(&adjustment_map);
                            sourcemap.set_debug_id(Some(debug_id));

                            let sourcemap_file = self.sources.get_mut(&sourcemap_url).unwrap();
                            sourcemap_file.contents.clear();
                            sourcemap.to_writer(&mut sourcemap_file.contents)?;

                            sourcemap_file.set_debug_id(debug_id.to_string());

                            if !dry_run {
                                let mut file = std::fs::File::create(&sourcemap_file.path)?;
                                file.write_all(&sourcemap_file.contents).context(format!(
                                    "Failed to write sourcemap file {}",
                                    sourcemap_file.path.display()
                                ))?;
                            }

                            if debug_id_fresh {
                                report
                                    .sourcemaps
                                    .push((sourcemap_file.path.clone(), debug_id));
                            } else {
                                report
                                    .skipped_sourcemaps
                                    .push((sourcemap_file.path.clone(), debug_id));
                            }

                            debug_id
                        } else {
                            // Case 4: We have a URL for the external sourcemap, but we can't find it.
                            // This is substantially the same as case 1.
                            debug!("Sourcemap file {} not found", sourcemap_url);
                            // source map cannot be found, fall back to hashing the contents.
                            let source_file = self.sources.get_mut(source_url).unwrap();
                            let debug_id =
                                inject::debug_id_from_bytes_hashed(&source_file.contents);

                            // If we don't have a sourcemap, it's not safe to inject the code snippet at the beginning,
                            // because that would throw off all the mappings. Instead, inject the snippet at the very end.
                            // This isn't ideal, but it's the best we can do in this case.
                            inject::fixup_js_file_end(&mut source_file.contents, debug_id)
                                .context(format!(
                                    "Failed to process {}",
                                    source_file.path.display()
                                ))?;

                            debug_id
                        }
                    }
                }
            };

            // Finally, some housekeeping.
            let source_file = self.sources.get_mut(source_url).unwrap();

            source_file.set_debug_id(debug_id.to_string());
            self.debug_ids.insert(source_url.clone(), debug_id);

            if !dry_run {
                let mut file = std::fs::File::create(&source_file.path)?;
                file.write_all(&source_file.contents).context(format!(
                    "Failed to write source file {}",
                    source_file.path.display()
                ))?;
            }

            report.injected.push((source_file.path.clone(), debug_id));
        }

        if !report.is_empty() {
            println!("{report}");
        } else {
            println!("> Nothing to inject")
        }

        Ok(())
    }
}

fn validate_script(source: &mut SourceFile) -> Result<()> {
    if let Some(sm_ref) = get_sourcemap_ref(source) {
        if let sourcemap::SourceMapRef::LegacyRef(_) = sm_ref {
            source.warn("encountered a legacy reference".into());
        }
        let url = sm_ref.get_url();
        if source.url.starts_with('/') {
            let full_url = Path::new(&source.url).join(url);
            info!(
                "found sourcemap for {} at {}",
                &source.url,
                full_url.display()
            );
        } else {
            let full_url = join_url(&source.url, url)?;
            info!("found sourcemap for {} at {}", &source.url, full_url);
        };
    } else if source.ty == SourceFileType::MinifiedSource {
        source.error("missing sourcemap!".into());
    }

    Ok(())
}

fn validate_regular(
    source_urls: &HashSet<String>,
    source: &mut SourceFile,
    sm: &sourcemap::SourceMap,
) {
    for idx in 0..sm.get_source_count() {
        let source_url = sm.get_source(idx).unwrap_or("??");
        if sm.get_source_contents(idx).is_some() || source_urls.contains(source_url) {
            info!("validator found source ({})", source_url);
        } else {
            source.warn(format!("missing sourcecode ({source_url})"));
        }
    }
}

fn validate_sourcemap(source_urls: &HashSet<String>, source: &mut SourceFile) -> Result<()> {
    match sourcemap::decode_slice(&source.contents)? {
        sourcemap::DecodedMap::Hermes(smh) => validate_regular(source_urls, source, &smh),
        sourcemap::DecodedMap::Regular(sm) => validate_regular(source_urls, source, &sm),
        sourcemap::DecodedMap::Index(_) => {
            source.warn("encountered indexed sourcemap. We cannot validate those.".into());
        }
    }
    Ok(())
}

impl Default for SourceMapProcessor {
    fn default() -> Self {
        SourceMapProcessor::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_join() {
        assert_eq!(&join_url("app:///", "foo.html").unwrap(), "app:///foo.html");
        assert_eq!(&join_url("app://", "foo.html").unwrap(), "app:///foo.html");
        assert_eq!(&join_url("~/", "foo.html").unwrap(), "~/foo.html");
        assert_eq!(
            &join_url("app:///", "/foo.html").unwrap(),
            "app:///foo.html"
        );
        assert_eq!(&join_url("app://", "/foo.html").unwrap(), "app:///foo.html");
        assert_eq!(
            &join_url("https:///example.com/", "foo.html").unwrap(),
            "https://example.com/foo.html"
        );
        assert_eq!(
            &join_url("https://example.com/", "foo.html").unwrap(),
            "https://example.com/foo.html"
        );
    }

    #[test]
    fn test_url_matches_extension() {
        assert!(url_matches_extension("foo.js", &["js"][..]));
        assert!(!url_matches_extension("foo.mjs", &["js"][..]));
        assert!(url_matches_extension("foo.mjs", &["js", "mjs"][..]));
        assert!(!url_matches_extension("js", &["js"][..]));
        assert!(url_matches_extension("foo.test.js", &["test.js"][..]));
    }
}
