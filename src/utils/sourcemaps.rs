//! Provides sourcemap validation functionality.
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io::Write;
use std::mem;
use std::path::{Path, PathBuf};
use std::str;
use std::str::FromStr;

use anyhow::{bail, Context, Error, Result};
use console::style;
use indicatif::ProgressStyle;
use log::{debug, info, warn};
use sha1_smol::Digest;
use symbolic::debuginfo::js::{
    discover_debug_id, discover_sourcemap_embedded_debug_id, discover_sourcemaps_location,
};
use symbolic::debuginfo::sourcebundle::SourceFileType;
use url::Url;

use crate::api::Api;
use crate::utils::enc::decode_unknown_string;
use crate::utils::file_search::ReleaseFileMatch;
use crate::utils::file_upload::{
    initialize_legacy_release_upload, FileUpload, SourceFile, SourceFiles, UploadContext,
};
use crate::utils::logging::is_quiet_mode;
use crate::utils::progress::ProgressBar;
use crate::utils::sourcemaps::inject::{fixup_js_file, normalize_sourcemap_url, InjectReport};

pub mod inject;

fn is_likely_minified_js(code: &[u8]) -> bool {
    // if we have a debug id or source maps location reference, this is a minified file
    if let Ok(code) = std::str::from_utf8(code) {
        if discover_debug_id(code).is_some() || discover_sourcemaps_location(code).is_some() {
            return true;
        }
    }
    if let Ok(code_str) = decode_unknown_string(code) {
        might_be_minified::analyze_str(&code_str).is_likely_minified()
    } else {
        false
    }
}

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
    get_sourcemap_reference_from_headers(file.headers.iter().map(|(k, v)| (k, v)))
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

fn guess_sourcemap_reference(sourcemaps: &HashSet<String>, min_url: &str) -> Result<String> {
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
        let new_ext = format!("{ext}.{map_ext}");
        if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
            return Ok(unsplit_url(None, basename, Some(&new_ext)));
        }

        // foo.min.js -> foo.js.map
        if let Some(rest) = ext.strip_prefix("min.") {
            let new_ext = format!("{rest}.{map_ext}");
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(unsplit_url(None, basename, Some(&new_ext)));
            }
        }

        // foo.min.js -> foo.min.map
        let mut parts: Vec<_> = ext.split('.').collect();
        if parts.len() > 1 {
            let parts_len = parts.len();
            parts[parts_len - 1] = map_ext;
            let new_ext = parts.join(".");
            if sourcemaps.contains(&unsplit_url(path, basename, Some(&new_ext))) {
                return Ok(unsplit_url(None, basename, Some(&new_ext)));
            }
        }
    }

    bail!("Could not auto-detect referenced sourcemap for {}", min_url);
}

pub struct SourceMapProcessor {
    pending_sources: HashSet<(String, ReleaseFileMatch)>,
    sources: SourceFiles,
}

fn is_hermes_bytecode(slice: &[u8]) -> bool {
    // The hermes bytecode format magic is defined here:
    // https://github.com/facebook/hermes/blob/5243222ef1d92b7393d00599fc5cff01d189a88a/include/hermes/BCGen/HBC/BytecodeFileFormat.h#L24-L25
    const HERMES_MAGIC: [u8; 8] = [0xC6, 0x1F, 0xBC, 0x03, 0xC1, 0x03, 0x19, 0x1F];
    slice.starts_with(&HERMES_MAGIC)
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
            } else if file
                .path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|x| x.contains(".min."))
                .unwrap_or(false)
                || is_likely_minified_js(&file.contents)
            {
                (
                    SourceFileType::MinifiedSource,
                    std::str::from_utf8(&file.contents)
                        .ok()
                        .and_then(discover_debug_id),
                )
            } else if is_hermes_bytecode(&file.contents) {
                // This is actually a big hack:
                // For the react-native Hermes case, we skip uploading the bytecode bundle,
                // and rather flag it as an empty "minified source". That way, it
                // will get a SourceMap reference, and the server side processor
                // should deal with it accordingly.
                file.contents.clear();
                (SourceFileType::MinifiedSource, None)
            } else {
                (SourceFileType::Source, None)
            };

            // attach the debug id to the artifact bundle when it's detected
            let mut headers = Vec::new();
            if let Some(debug_id) = debug_id {
                headers.push(("debug-id".to_string(), debug_id.to_string()));
            }

            self.sources.insert(
                url.clone(),
                SourceFile {
                    url: url.clone(),
                    path: file.path,
                    contents: file.contents,
                    ty,
                    headers,
                    messages: vec![],
                    already_uploaded: false,
                },
            );
            pb.inc(1);
        }

        pb.finish_with_duration("Analyzing");
    }

    pub fn dump_log(&self, title: &str) {
        if is_quiet_mode() {
            return;
        }

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

            if source.already_uploaded {
                println!(
                    "    {}",
                    style(format!("{} (skipped; already uploaded)", &source.url)).yellow()
                );
                continue;
            }

            let mut pieces = Vec::new();

            if source.ty == SourceFileType::MinifiedSource {
                if let Some(sm_ref) = get_sourcemap_ref(source) {
                    pieces.push(format!("sourcemap at {}", style(sm_ref.get_url()).cyan()));
                } else {
                    pieces.push("no sourcemap ref".into());
                }
            }
            if let Some((_, debug_id)) = source.headers.iter().find(|x| x.0 == "debug-id") {
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
            sourcemap::DecodedMap::Index(sourcemap_index) => sourcemap_index,
            _ => {
                warn!("Invalid sourcemap type for RAM bundle, skipping");
                return Ok(());
            }
        };

        // We don't need the RAM bundle sourcemap itself
        self.sources.remove(&sourcemap_url);

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
                    headers: vec![],
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
                    headers: vec![],
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
        let sourcemaps = self
            .sources
            .iter()
            .map(|x| x.1)
            .filter(|x| x.ty == SourceFileType::SourceMap)
            .map(|x| x.url.to_string())
            .collect();

        println!("{} Adding source map references", style(">").dim());
        for source in self.sources.values_mut() {
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }

            // If there is already an embedded reference, use it.
            if let Ok(contents) = std::str::from_utf8(&source.contents) {
                if let Some(target_url) = discover_sourcemaps_location(contents) {
                    source
                        .headers
                        .push(("Sourcemap".to_string(), target_url.to_string()));
                    continue;
                }
            }

            // we silently ignore when we can't find a sourcemap. Maybe we should
            // log this.
            match guess_sourcemap_reference(&sourcemaps, &source.url) {
                Ok(target_url) => {
                    source.headers.push(("Sourcemap".to_string(), target_url));
                }
                Err(err) => {
                    source.warn(format!(
                        "could not determine a source map reference ({err})"
                    ));
                }
            }
        }
        Ok(())
    }

    fn flag_uploaded_sources(&mut self, context: &UploadContext<'_>) {
        // TODO: this endpoint does not exist for non release based uploads
        if !context.dedupe {
            return;
        }
        let release = match context.release {
            Some(release) => release,
            None => return,
        };

        let mut sources_checksums: Vec<_> = self
            .sources
            .values()
            .filter_map(|s| s.checksum().map(|c| c.to_string()).ok())
            .collect();

        // Checksums need to be sorted in order to satisfy integration tests constraints.
        sources_checksums.sort();

        let api = Api::current();

        if let Ok(artifacts) = api.list_release_files_by_checksum(
            context.org,
            context.project,
            release,
            &sources_checksums,
        ) {
            let already_uploaded_checksums: Vec<_> = artifacts
                .iter()
                .filter_map(|artifact| Digest::from_str(&artifact.sha1).ok())
                .collect();

            for mut source in self.sources.values_mut() {
                if let Ok(checksum) = source.checksum() {
                    if already_uploaded_checksums.contains(&checksum) {
                        source.already_uploaded = true;
                    }
                }
            }
        }
    }

    /// Uploads all files
    pub fn upload(&mut self, context: &UploadContext<'_>) -> Result<()> {
        initialize_legacy_release_upload(context)?;
        self.flush_pending_sources();
        self.flag_uploaded_sources(context);
        let mut uploader = FileUpload::new(context);
        uploader.files(&self.sources);
        uploader.upload()?;
        self.dump_log("Source Map Upload Report");
        Ok(())
    }

    /// Injects debug ids into minified source files and sourcemaps.
    ///
    /// This iterates over contained minified source files and their referenced
    /// sourcemaps and ties them together with debug ids (either freshly generated
    /// or already contained in the sourcemap).
    ///
    /// If `dry_run` is false, this will modify the source and sourcemap files on disk!
    pub fn inject_debug_ids(&mut self, dry_run: bool) -> Result<()> {
        self.flush_pending_sources();
        let sourcemaps = self
            .sources
            .iter()
            .filter_map(|(_, source)| {
                (source.ty == SourceFileType::SourceMap).then(|| source.url.to_string())
            })
            .collect();

        println!("{} Injecting debug ids", style(">").dim());

        let mut report = InjectReport::default();

        // Step 1: find all references from minified source files to sourcemaps
        let mut sourcemap_refs = Vec::new();

        for source in self.sources.values() {
            if source.ty != SourceFileType::MinifiedSource {
                continue;
            }

            if let Ok(contents) = std::str::from_utf8(&source.contents) {
                if let Some(debug_id) = discover_debug_id(contents) {
                    debug!("File {} was previously processed", source.path.display());
                    report
                        .previously_injected
                        .push((source.path.clone(), debug_id));
                    continue;
                }

                let sourcemap_url = match discover_sourcemaps_location(contents) {
                    Some(url) => url.to_string(),
                    None => match guess_sourcemap_reference(&sourcemaps, &source.url) {
                        Ok(url) => url,
                        Err(_) => {
                            report.skipped.push(source.path.clone());
                            continue;
                        }
                    },
                };

                let sourcemap_url = normalize_sourcemap_url(&source.url, &sourcemap_url);

                sourcemap_refs.push((source.url.clone(), sourcemap_url));
            }
        }

        // Step 2: Produce a debug id for each source file by either reading it from the
        // sourcemap if its already there or generating a fresh one and writing it to the sourcemap
        // if it isn't.
        let mut debug_ids = Vec::new();

        for (source_url, sourcemap_url) in sourcemap_refs {
            let Some(sourcemap_file) = self.sources.get_mut(&sourcemap_url) else {
                debug!("Sourcemap file {} not found", sourcemap_url);
                report.missing_sourcemaps.push(source_url.into());
                continue;
            };

            let (debug_id, sourcemap_modified) =
                inject::fixup_sourcemap(&mut sourcemap_file.contents).context(format!(
                    "Failed to process {}",
                    sourcemap_file.path.display()
                ))?;

            sourcemap_file
                .headers
                .push(("debug-id".to_string(), debug_id.to_string()));

            if !dry_run && sourcemap_modified {
                let mut file = std::fs::File::options()
                    .write(true)
                    .open(&sourcemap_file.path)?;
                file.write_all(&sourcemap_file.contents).context(format!(
                    "Failed to write sourcemap file {}",
                    sourcemap_file.path.display()
                ))?;
            }

            if sourcemap_modified {
                report
                    .sourcemaps
                    .push((sourcemap_file.path.clone(), debug_id));
            } else {
                report
                    .skipped_sourcemaps
                    .push((sourcemap_file.path.clone(), debug_id));
            }

            debug_ids.push((source_url, debug_id));
        }

        // Step 3: Iterate over the minified sourcemaps again and inject the debug ids.
        for (source_url, debug_id) in debug_ids {
            let source_file = self.sources.get_mut(&source_url).unwrap();

            fixup_js_file(&mut source_file.contents, debug_id)
                .context(format!("Failed to process {}", source_file.path.display()))?;

            source_file
                .headers
                .push(("debug-id".to_string(), debug_id.to_string()));

            if !dry_run {
                let mut file = std::fs::File::options()
                    .write(true)
                    .open(&source_file.path)?;
                file.write_all(&source_file.contents).context(format!(
                    "Failed to write source file {}",
                    source_file.path.display()
                ))?;
            }

            report.injected.push((source_file.path.clone(), debug_id));
        }

        if !report.is_empty() {
            println!("{report}");
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
