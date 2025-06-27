//! Searches, processes and uploads release files.
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::io::BufWriter;
use std::path::PathBuf;
use std::str;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use console::style;
use log::info;
use parking_lot::RwLock;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use sentry::types::DebugId;
use sha1_smol::Digest;
use symbolic::common::ByteView;
use symbolic::debuginfo::js;
use symbolic::debuginfo::sourcebundle::{
    SourceBundleErrorKind, SourceBundleWriter, SourceFileInfo, SourceFileType,
};
use thiserror::Error;
use url::Url;

use crate::api::NewRelease;
use crate::api::{Api, ChunkServerOptions, ChunkUploadCapability};
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::fs::{get_sha1_checksum, get_sha1_checksums, TempFile};
use crate::utils::progress::{ProgressBar, ProgressBarMode, ProgressStyle};

use super::file_search::ReleaseFileMatch;

/// Fallback concurrency for release file uploads.
static DEFAULT_CONCURRENCY: usize = 4;

/// Old versions of Sentry cannot assemble artifact bundles straight away, they require
/// that those bundles are associated to a release.
///
/// This function checks whether the configured server supports artifact bundles
/// and only creates a release if the server requires that.
pub fn initialize_legacy_release_upload(context: &UploadContext) -> Result<()> {
    // if the remote sentry service supports artifact bundles, we don't
    // need to do anything here.  Artifact bundles will also only work
    // if a project is provided which is technically unnecessary for the
    // legacy upload though it will unlikely to be what users want.
    if !context.projects.is_empty()
        && context.chunk_upload_options.is_some_and(|x| {
            x.supports(ChunkUploadCapability::ArtifactBundles)
                || x.supports(ChunkUploadCapability::ArtifactBundlesV2)
        })
    {
        return Ok(());
    }

    // TODO: make this into an error later down the road
    if context.projects.is_empty() {
        eprintln!(
            "{}",
            style(
                "warning: no project specified. \
                    While this upload will succeed it will be unlikely that \
                    this is what you wanted. Future versions of sentry will \
                    require a project to be set."
            )
            .red()
        );
    }

    if let Some(version) = context.release {
        let api = Api::current();
        api.authenticated()?.new_release(
            context.org,
            &NewRelease {
                version: version.to_string(),
                projects: context.projects.to_vec(),
                ..Default::default()
            },
        )?;
    } else {
        bail!("This version of Sentry does not support artifact bundles. A release slug is required (provide with --release or by setting the SENTRY_RELEASE environment variable)");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct UploadContext<'a> {
    pub org: &'a str,
    pub projects: &'a [String],
    pub release: Option<&'a str>,
    pub dist: Option<&'a str>,
    pub note: Option<&'a str>,
    pub wait: bool,
    pub max_wait: Duration,
    pub dedupe: bool,
    pub chunk_upload_options: Option<&'a ChunkServerOptions>,
}

impl UploadContext<'_> {
    pub fn release(&self) -> Result<&str> {
        self.release
            .ok_or_else(|| anyhow!("A release slug is required (provide with --release or by setting the SENTRY_RELEASE environment variable)"))
    }
}

#[derive(Debug, Error)]
pub enum LegacyUploadContextError {
    #[error("a release is required for this upload")]
    ReleaseMissing,
    #[error("only a single project is supported for this upload")]
    ProjectMultiple,
}

/// Represents the context for legacy release uploads.
///
/// `LegacyUploadContext` contains information needed for legacy (non-chunked)
/// uploads. Legacy uploads are primarily used when uploading to old self-hosted
/// Sentry servers, which do not support receiving chunked uploads.
///
/// Unlike chunked uploads, legacy uploads require a release to be set,
/// and do not need to have chunk-upload-related fields.
#[derive(Debug, Default)]
pub struct LegacyUploadContext<'a> {
    org: &'a str,
    project: Option<&'a str>,
    release: &'a str,
    dist: Option<&'a str>,
}

impl LegacyUploadContext<'_> {
    pub fn org(&self) -> &str {
        self.org
    }

    pub fn project(&self) -> Option<&str> {
        self.project
    }

    pub fn release(&self) -> &str {
        self.release
    }

    pub fn dist(&self) -> Option<&str> {
        self.dist
    }
}

impl Display for LegacyUploadContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} {}",
            style("> Organization:").dim(),
            style(self.org).yellow()
        )?;
        writeln!(
            f,
            "{} {}",
            style("> Project:").dim(),
            style(self.project.unwrap_or("None")).yellow()
        )?;
        writeln!(
            f,
            "{} {}",
            style("> Release:").dim(),
            style(self.release).yellow()
        )?;
        writeln!(
            f,
            "{} {}",
            style("> Dist:").dim(),
            style(self.dist.unwrap_or("None")).yellow()
        )?;
        write!(
            f,
            "{} {}",
            style("> Upload type:").dim(),
            style("single file/legacy upload").yellow()
        )
    }
}

impl<'a> TryFrom<&'a UploadContext<'_>> for LegacyUploadContext<'a> {
    type Error = LegacyUploadContextError;

    fn try_from(value: &'a UploadContext) -> Result<Self, Self::Error> {
        let &UploadContext {
            org,
            projects,
            release,
            dist,
            ..
        } = value;

        let project = match projects {
            [] => None,
            [project] => Some(project.as_str()),
            [_, _, ..] => Err(LegacyUploadContextError::ProjectMultiple)?,
        };

        let release = release.ok_or(LegacyUploadContextError::ReleaseMissing)?;

        Ok(Self {
            org,
            project,
            release,
            dist,
        })
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum LogLevel {
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

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub url: String,
    pub path: PathBuf,
    pub contents: Arc<Vec<u8>>,
    pub ty: SourceFileType,
    /// A map of headers attached to the source file.
    ///
    /// Headers that `sentry-cli` knows about are
    /// * "debug-id" for a file's debug id
    /// * "Sourcemap" for a reference to a file's sourcemap
    pub headers: BTreeMap<String, String>,
    pub messages: Vec<(LogLevel, String)>,
    pub already_uploaded: bool,
}

impl SourceFile {
    pub fn from_release_file_match(url: &str, mut file: ReleaseFileMatch) -> SourceFile {
        let (ty, debug_id) = if sourcemap::is_sourcemap_slice(&file.contents) {
            (
                SourceFileType::SourceMap,
                std::str::from_utf8(&file.contents)
                    .ok()
                    .and_then(js::discover_sourcemap_embedded_debug_id),
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
                    .and_then(js::discover_debug_id),
            )
        };

        let mut source_file = SourceFile {
            url: url.into(),
            path: file.path,
            contents: file.contents.into(),
            ty,
            headers: BTreeMap::new(),
            messages: vec![],
            already_uploaded: false,
        };

        if let Some(debug_id) = debug_id {
            source_file.set_debug_id(debug_id.to_string());
        }
        source_file
    }

    /// Calculates and returns the SHA1 checksum of the file.
    pub fn checksum(&self) -> Result<Digest> {
        get_sha1_checksum(&**self.contents)
    }

    /// Returns the value of the "debug-id" header.
    pub fn debug_id(&self) -> Option<&String> {
        self.headers.get("debug-id")
    }

    /// Sets the value of the "debug-id" header.
    pub fn set_debug_id(&mut self, debug_id: String) {
        self.headers.insert("debug-id".to_string(), debug_id);
    }

    /// Sets the value of the "Sourcemap" header.
    pub fn set_sourcemap_reference(&mut self, sourcemap: String) {
        self.headers.insert("Sourcemap".to_string(), sourcemap);
    }

    pub fn log(&mut self, level: LogLevel, msg: String) {
        self.messages.push((level, msg));
    }

    pub fn warn(&mut self, msg: String) {
        self.log(LogLevel::Warning, msg);
    }

    pub fn error(&mut self, msg: String) {
        self.log(LogLevel::Error, msg);
    }
}

/// A map from URLs to source files.
///
/// The keys correspond to the `url` field on the values.
pub type SourceFiles = BTreeMap<String, SourceFile>;

pub struct FileUpload<'a> {
    context: &'a UploadContext<'a>,
    files: SourceFiles,
}

impl<'a> FileUpload<'a> {
    pub fn new(context: &'a UploadContext) -> Self {
        FileUpload {
            context,
            files: SourceFiles::new(),
        }
    }

    pub fn files(&mut self, files: &SourceFiles) -> &mut Self {
        for (k, v) in files {
            if !v.already_uploaded {
                self.files.insert(k.to_owned(), v.to_owned());
            }
        }
        self
    }

    pub fn upload(&self) -> Result<()> {
        // multiple projects OK
        initialize_legacy_release_upload(self.context)?;

        if let Some(chunk_options) = self.context.chunk_upload_options {
            if chunk_options.supports(ChunkUploadCapability::ReleaseFiles) {
                // multiple projects OK
                return upload_files_chunked(self.context, &self.files, chunk_options);
            }
        }

        log::warn!(
            "Your Sentry server does not support chunked uploads. \
            We are falling back to a legacy upload method, which \
            has fewer features and is less reliable. Please consider \
            upgrading your Sentry server or switching to our SaaS offering."
        );

        // Do not permit uploads of more than 20k files if the server does not
        // support artifact bundles.  This is a temporary downside protection to
        // protect users from uploading more sources than we support.
        if self.files.len() > 20_000 {
            bail!(
                "Too many sources: {} exceeds maximum allowed files per release",
                &self.files.len()
            );
        }

        let concurrency = self
            .context
            .chunk_upload_options
            .map_or(DEFAULT_CONCURRENCY, |o| usize::from(o.concurrency));

        let legacy_context = &self.context.try_into().map_err(|e| {
            anyhow::anyhow!(
                "Error while performing legacy upload: {e}. \
                If you would like to upload files {}, you need to upgrade your Sentry server \
                or switch to our SaaS offering.",
                match e {
                    LegacyUploadContextError::ReleaseMissing => "without specifying a release",
                    LegacyUploadContextError::ProjectMultiple =>
                        "to multiple projects simultaneously",
                }
            )
        })?;

        upload_files_parallel(legacy_context, &self.files, concurrency)
    }

    pub fn build_jvm_bundle(&self, debug_id: Option<DebugId>) -> Result<TempFile> {
        build_artifact_bundle(self.context, &self.files, debug_id)
    }
}

fn upload_files_parallel(
    context: &LegacyUploadContext,
    files: &SourceFiles,
    num_threads: usize,
) -> Result<()> {
    let api = Api::current();
    let release = context.release();

    // get a list of release files first so we know the file IDs of
    // files that already exist.
    let release_files: HashMap<_, _> = api
        .authenticated()?
        .list_release_files(context.org, context.project(), release)?
        .into_iter()
        .map(|artifact| ((artifact.dist, artifact.name), artifact.id))
        .collect();

    println!(
        "{} Uploading source maps for release {}",
        style(">").dim(),
        style(release).cyan()
    );

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "{} Uploading {} source map{}...\
     \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
        style(">").dim(),
        style(files.len().to_string()).yellow(),
        if files.len() == 1 { "" } else { "s" }
    ));

    let total_bytes = files.values().map(|file| file.contents.len()).sum();
    let files = files.iter().collect::<Vec<_>>();

    let pb = Arc::new(ProgressBar::new(total_bytes));
    pb.set_style(progress_style);

    let pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
    let bytes = Arc::new(RwLock::new(vec![0u64; files.len()]));

    pool.install(|| {
        files
            .into_par_iter()
            .enumerate()
            .map(|(index, (_, file))| -> Result<()> {
                let api = Api::current();
                let authenticated_api = api.authenticated()?;
                let mode = ProgressBarMode::Shared((
                    pb.clone(),
                    file.contents.len() as u64,
                    index,
                    bytes.clone(),
                ));

                if let Some(old_id) =
                    release_files.get(&(context.dist.map(|x| x.into()), file.url.clone()))
                {
                    authenticated_api
                        .delete_release_file(context.org, context.project, release, old_id)
                        .ok();
                }

                authenticated_api
                    .region_specific(context.org)
                    .upload_release_file(
                        context,
                        &file.contents,
                        &file.url,
                        Some(
                            file.headers
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                        mode,
                    )?;

                Ok(())
            })
            .collect::<Result<(), _>>()
    })?;

    pb.finish_and_clear();

    println!("{context}");

    Ok(())
}

fn poll_assemble(
    checksum: Digest,
    chunks: &[Digest],
    context: &UploadContext,
    options: &ChunkServerOptions,
) -> Result<()> {
    let progress_style = ProgressStyle::default_spinner().template("{spinner} Processing files...");

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let assemble_start = Instant::now();
    let options_max_wait = match options.max_wait {
        0 => DEFAULT_MAX_WAIT,
        secs => Duration::from_secs(secs),
    };

    let max_wait = context.max_wait.min(options_max_wait);

    let api = Api::current();
    let authenticated_api = api.authenticated()?;
    let use_artifact_bundle = (options.supports(ChunkUploadCapability::ArtifactBundles)
        || options.supports(ChunkUploadCapability::ArtifactBundlesV2))
        && !context.projects.is_empty();
    let response = loop {
        // prefer standalone artifact bundle upload over legacy release based upload
        let response = if use_artifact_bundle {
            authenticated_api.assemble_artifact_bundle(
                context.org,
                context.projects,
                checksum,
                chunks,
                context.release,
                context.dist,
            )?
        } else {
            authenticated_api.assemble_release_artifacts(
                context.org,
                context.release()?,
                checksum,
                chunks,
            )?
        };

        // Poll until there is a response, unless the user has specified to skip polling. In
        // that case, we return the potentially partial response from the server. This might
        // still contain a cached error.
        if !context.wait || response.state.is_finished() {
            break response;
        }

        if assemble_start.elapsed() > max_wait {
            break response;
        }

        std::thread::sleep(ASSEMBLE_POLL_INTERVAL);
    };

    if response.state.is_err() {
        let message = response.detail.as_deref().unwrap_or("unknown error");
        bail!("Failed to process uploaded files: {}", message);
    }

    pb.finish_with_duration("Processing");

    if response.state.is_pending() {
        if context.wait {
            bail!("Failed to process files in {}s", max_wait.as_secs());
        } else {
            println!(
                "{} File upload complete (processing pending on server)",
                style(">").dim()
            );
        }
    } else {
        println!("{} File processing complete", style(">").dim());
    }

    print_upload_context_details(context);

    Ok(())
}

fn upload_files_chunked(
    context: &UploadContext,
    files: &SourceFiles,
    options: &ChunkServerOptions,
) -> Result<()> {
    let archive = build_artifact_bundle(context, files, None)?;

    let progress_style =
        ProgressStyle::default_spinner().template("{spinner} Optimizing bundle for upload...");

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let view = ByteView::open(archive.path())?;
    let (checksum, checksums) = get_sha1_checksums(&view, options.chunk_size as usize)?;
    let mut chunks = view
        .chunks(options.chunk_size as usize)
        .zip(checksums.iter())
        .map(|(data, checksum)| Chunk((*checksum, data)))
        .collect::<Vec<_>>();

    pb.finish_with_duration("Optimizing");

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "{} Uploading files...\
       \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
        style(">").dim(),
    ));

    // Filter out chunks that are already on the server. This only matters if the server supports
    // `ArtifactBundlesV2`, otherwise the `missing_chunks` field is meaningless.
    if options.supports(ChunkUploadCapability::ArtifactBundlesV2) && !context.projects.is_empty() {
        let api = Api::current();
        let response = api.authenticated()?.assemble_artifact_bundle(
            context.org,
            context.projects,
            checksum,
            &checksums,
            context.release,
            context.dist,
        )?;
        chunks.retain(|Chunk((digest, _))| response.missing_chunks.contains(digest));
    };

    if !chunks.is_empty() {
        upload_chunks(&chunks, options, progress_style)?;
        println!("{} Uploaded files to Sentry", style(">").dim());
    } else {
        println!(
            "{} Nothing to upload, all files are on the server",
            style(">").dim()
        );
    }
    poll_assemble(checksum, &checksums, context, options)
}

/// Creates a debug id from a map of source files by hashing each file's
/// URL, contents, type, and headers.
fn build_debug_id(files: &SourceFiles) -> DebugId {
    let mut hash = sha1_smol::Sha1::new();
    for source_file in files.values() {
        hash.update(source_file.url.as_bytes());
        hash.update(&source_file.contents);
        hash.update(format!("{:?}", source_file.ty).as_bytes());

        for (key, value) in &source_file.headers {
            hash.update(key.as_bytes());
            hash.update(value.as_bytes());
        }
    }

    let mut sha1_bytes = [0u8; 16];
    sha1_bytes.copy_from_slice(&hash.digest().bytes()[..16]);
    DebugId::from_uuid(uuid::Builder::from_sha1_bytes(sha1_bytes).into_uuid())
}

fn build_artifact_bundle(
    context: &UploadContext,
    files: &SourceFiles,
    debug_id: Option<DebugId>,
) -> Result<TempFile> {
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Bundling files for upload... {msg:.dim}\
       \n{wide_bar}  {pos}/{len}",
    );

    let pb = ProgressBar::new(files.len());
    pb.set_style(progress_style);
    pb.set_prefix(">");

    let archive = TempFile::create()?;
    let mut bundle = SourceBundleWriter::start(BufWriter::new(archive.open()?))?;

    // artifact bundles get a random UUID as debug id
    let debug_id = debug_id.unwrap_or_else(|| build_debug_id(files));
    bundle.set_attribute("debug_id", debug_id.to_string());

    if let Some(note) = context.note {
        bundle.set_attribute("note", note.to_owned());
    }

    bundle.set_attribute("org".to_owned(), context.org.to_owned());
    if let [project] = context.projects {
        // Only set project if there is exactly one project
        bundle.set_attribute("project".to_owned(), project);
    }
    if let Some(release) = context.release {
        bundle.set_attribute("release".to_owned(), release.to_owned());
    }
    if let Some(dist) = context.dist {
        bundle.set_attribute("dist".to_owned(), dist.to_owned());
    }

    let mut bundle_file_count = 0;

    for file in files.values() {
        pb.inc(1);
        pb.set_message(&file.url);

        let mut info = SourceFileInfo::new();
        info.set_ty(file.ty);
        info.set_url(file.url.clone());
        for (k, v) in &file.headers {
            info.add_header(k.clone(), v.clone());
        }

        let bundle_path = url_to_bundle_path(&file.url)?;
        if let Err(e) = bundle.add_file(bundle_path, file.contents.as_slice(), info) {
            if e.kind() == SourceBundleErrorKind::ReadFailed {
                info!(
                    "Skipping {} because it is not valid UTF-8.",
                    file.path.display()
                );
                continue;
            } else {
                return Err(e.into());
            }
        }
        bundle_file_count += 1;
    }

    bundle.finish()?;

    pb.finish_with_duration("Bundling");

    println!(
        "{} Bundled {} {} for upload",
        style(">").dim(),
        style(bundle_file_count).yellow(),
        match bundle_file_count {
            1 => "file",
            _ => "files",
        }
    );

    println!(
        "{} Bundle ID: {}",
        style(">").dim(),
        style(debug_id).yellow(),
    );

    Ok(archive)
}

fn url_to_bundle_path(url: &str) -> Result<String> {
    let base = Url::parse("http://~").expect("this url is valid");
    let url = if let Some(rest) = url.strip_prefix("~/") {
        base.join(rest)?
    } else {
        base.join(url)?
    };

    let mut path = url.path().to_string();
    if let Some(fragment) = url.fragment() {
        path = format!("{path}#{fragment}");
    }
    if path.starts_with('/') {
        path.remove(0);
    }

    Ok(match url.host_str() {
        Some("~") => format!("_/_/{path}"),
        Some(host) => format!("{}/{}/{}", url.scheme(), host, path),
        None => format!("{}/_/{}", url.scheme(), path),
    })
}

fn print_upload_context_details(context: &UploadContext) {
    println!(
        "{} {}",
        style("> Organization:").dim(),
        style(context.org).yellow()
    );
    println!(
        "{} {}",
        style("> Projects:").dim(),
        style(context.projects.join(", ")).yellow()
    );
    println!(
        "{} {}",
        style("> Release:").dim(),
        style(context.release.unwrap_or("None")).yellow()
    );
    println!(
        "{} {}",
        style("> Dist:").dim(),
        style(context.dist.unwrap_or("None")).yellow()
    );
    let upload_type = match context.chunk_upload_options {
        None => "single file",
        Some(opts)
            if opts.supports(ChunkUploadCapability::ArtifactBundles)
                || opts.supports(ChunkUploadCapability::ArtifactBundlesV2) =>
        {
            "artifact bundle"
        }
        _ => "release bundle",
    };
    println!(
        "{} {}",
        style("> Upload type:").dim(),
        style(upload_type).yellow()
    );
}

fn is_hermes_bytecode(slice: &[u8]) -> bool {
    // The hermes bytecode format magic is defined here:
    // https://github.com/facebook/hermes/blob/5243222ef1d92b7393d00599fc5cff01d189a88a/include/hermes/BCGen/HBC/BytecodeFileFormat.h#L24-L25
    const HERMES_MAGIC: [u8; 8] = [0xC6, 0x1F, 0xBC, 0x03, 0xC1, 0x03, 0x19, 0x1F];
    slice.starts_with(&HERMES_MAGIC)
}

#[cfg(test)]
mod tests {
    use sha1_smol::Sha1;

    use super::*;

    #[test]
    fn test_url_to_bundle_path() {
        assert_eq!(url_to_bundle_path("~/bar").unwrap(), "_/_/bar");
        assert_eq!(url_to_bundle_path("~/foo/bar").unwrap(), "_/_/foo/bar");
        assert_eq!(
            url_to_bundle_path("~/dist/js/bundle.js.map").unwrap(),
            "_/_/dist/js/bundle.js.map"
        );
        assert_eq!(
            url_to_bundle_path("~/babel.config.js").unwrap(),
            "_/_/babel.config.js"
        );

        assert_eq!(url_to_bundle_path("~/#/bar").unwrap(), "_/_/#/bar");
        assert_eq!(url_to_bundle_path("~/foo/#/bar").unwrap(), "_/_/foo/#/bar");
        assert_eq!(
            url_to_bundle_path("~/dist/#js/bundle.js.map").unwrap(),
            "_/_/dist/#js/bundle.js.map"
        );
        assert_eq!(
            url_to_bundle_path("~/#foo/babel.config.js").unwrap(),
            "_/_/#foo/babel.config.js"
        );
    }

    #[test]
    fn build_artifact_bundle_deterministic() {
        let context = UploadContext {
            org: "wat-org",
            projects: &["wat-project".into()],
            release: None,
            dist: None,
            note: None,
            wait: false,
            max_wait: DEFAULT_MAX_WAIT,
            dedupe: true,
            chunk_upload_options: None,
        };

        let source_files = ["bundle.min.js.map", "vendor.min.js.map"]
            .into_iter()
            .map(|name| {
                let file = SourceFile {
                    url: format!("~/{name}"),
                    path: format!("tests/integration/_fixtures/{name}").into(),
                    contents: std::fs::read(format!("tests/integration/_fixtures/{name}"))
                        .unwrap()
                        .into(),
                    ty: SourceFileType::SourceMap,
                    headers: Default::default(),
                    messages: Default::default(),
                    already_uploaded: false,
                };
                (format!("~/{name}"), file)
            })
            .collect();

        let file = build_artifact_bundle(&context, &source_files, None).unwrap();

        let buf = std::fs::read(file.path()).unwrap();
        let hash = Sha1::from(buf);
        assert_eq!(
            hash.digest().to_string(),
            "f0e25ae149b711c510148e022ebc883ad62c7c4c"
        );
    }
}
