//! Searches, processes and uploads release files.
use std::collections::HashMap;
use std::fmt;
use std::io::BufWriter;
use std::path::PathBuf;
use std::str;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use console::style;
use parking_lot::RwLock;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use sentry::types::DebugId;
use sha1_smol::Digest;
use symbolic::common::ByteView;
use symbolic::debuginfo::sourcebundle::{SourceBundleWriter, SourceFileInfo, SourceFileType};
use url::Url;
use uuid::Uuid;

use crate::api::NewRelease;
use crate::api::{Api, ChunkUploadCapability, ChunkUploadOptions, ProgressBarMode};
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::chunks::{upload_chunks, Chunk, ASSEMBLE_POLL_INTERVAL};
use crate::utils::fs::{get_sha1_checksum, get_sha1_checksums, TempFile};
use crate::utils::progress::{ProgressBar, ProgressStyle};

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
    if context.project.is_some()
        && context.chunk_upload_options.map_or(false, |x| {
            x.supports(ChunkUploadCapability::ArtifactBundles)
        })
    {
        return Ok(());
    }

    // TODO: make this into an error later down the road
    if context.project.is_none() {
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
        api.new_release(
            context.org,
            &NewRelease {
                version: version.to_string(),
                projects: context.project.map(|x| x.to_string()).into_iter().collect(),
                ..Default::default()
            },
        )?;
    } else {
        bail!("This version of Sentry does not support artifact bundles. A release slug is required (provide with --release)");
    }
    Ok(())
}

pub struct UploadContext<'a> {
    pub org: &'a str,
    pub project: Option<&'a str>,
    pub release: Option<&'a str>,
    pub dist: Option<&'a str>,
    pub note: Option<&'a str>,
    pub wait: bool,
    pub dedupe: bool,
    pub chunk_upload_options: Option<&'a ChunkUploadOptions>,
}

impl<'a> UploadContext<'a> {
    pub fn release(&self) -> Result<&str> {
        self.release
            .ok_or_else(|| anyhow!("A release slug is required (provide with --release)"))
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
    pub contents: Vec<u8>,
    pub ty: SourceFileType,
    pub headers: Vec<(String, String)>,
    pub messages: Vec<(LogLevel, String)>,
    pub already_uploaded: bool,
}

impl SourceFile {
    /// Calculates and returns the SHA1 checksum of the file.
    pub fn checksum(&self) -> Result<Digest> {
        get_sha1_checksum(&*self.contents)
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

pub type SourceFiles = HashMap<String, SourceFile>;

pub struct FileUpload<'a> {
    context: &'a UploadContext<'a>,
    files: SourceFiles,
}

impl<'a> FileUpload<'a> {
    pub fn new(context: &'a UploadContext) -> Self {
        FileUpload {
            context,
            files: HashMap::new(),
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
        initialize_legacy_release_upload(self.context)?;

        if let Some(chunk_options) = self.context.chunk_upload_options {
            if chunk_options.supports(ChunkUploadCapability::ReleaseFiles) {
                return upload_files_chunked(self.context, &self.files, chunk_options);
            }
        }

        // Do not permit uploads of more than 20k files if the server does not
        // support artifact bundles.  This is a termporary downside protection to
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
        upload_files_parallel(self.context, &self.files, concurrency)
    }
}

fn upload_files_parallel(
    context: &UploadContext,
    files: &SourceFiles,
    num_threads: usize,
) -> Result<()> {
    let api = Api::current();
    let release = context.release()?;

    // get a list of release files first so we know the file IDs of
    // files that already exist.
    let release_files: HashMap<_, _> = api
        .list_release_files(context.org, context.project, release)?
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
                let mode = ProgressBarMode::Shared((
                    pb.clone(),
                    file.contents.len() as u64,
                    index,
                    bytes.clone(),
                ));

                if let Some(old_id) =
                    release_files.get(&(context.dist.map(|x| x.into()), file.url.clone()))
                {
                    api.delete_release_file(context.org, context.project, release, old_id)
                        .ok();
                }

                api.upload_release_file(
                    context,
                    &file.contents,
                    &file.url,
                    Some(file.headers.as_slice()),
                    mode,
                )?;

                Ok(())
            })
            .collect::<Result<(), _>>()
    })?;

    pb.finish_and_clear();

    print_upload_context_details(context);

    Ok(())
}

fn upload_files_chunked(
    context: &UploadContext,
    files: &SourceFiles,
    options: &ChunkUploadOptions,
) -> Result<()> {
    let archive = build_artifact_bundle(context, files)?;

    let progress_style =
        ProgressStyle::default_spinner().template("{spinner} Optimizing bundle for upload...");

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let view = ByteView::open(archive.path())?;
    let (checksum, checksums) = get_sha1_checksums(&view, options.chunk_size)?;
    let chunks = view
        .chunks(options.chunk_size as usize)
        .zip(checksums.iter())
        .map(|(data, checksum)| Chunk((*checksum, data)))
        .collect::<Vec<_>>();

    pb.finish_with_duration("Optimizing");

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "{} Uploading release files...\
       \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
        style(">").dim(),
    ));

    upload_chunks(&chunks, options, progress_style)?;
    println!("{} Uploaded release files to Sentry", style(">").dim(),);

    let progress_style = ProgressStyle::default_spinner().template("{spinner} Processing files...");

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(progress_style);

    let assemble_start = Instant::now();
    let max_wait = match options.max_wait {
        0 => DEFAULT_MAX_WAIT,
        secs => Duration::from_secs(secs),
    };

    let api = Api::current();
    let response = loop {
        // prefer standalone artifact bundle upload over legacy release based upload
        let response = if options.supports(ChunkUploadCapability::ArtifactBundles)
            && context.project.is_some()
        {
            api.assemble_artifact_bundle(
                context.org,
                vec![context.project.unwrap().to_string()],
                checksum,
                &checksums,
                context.release,
                context.dist,
            )?
        } else {
            api.assemble_release_artifacts(context.org, context.release()?, checksum, &checksums)?
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

fn build_artifact_bundle(context: &UploadContext, files: &SourceFiles) -> Result<TempFile> {
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
    bundle.set_attribute("debug_id", DebugId::from_uuid(Uuid::new_v4()).to_string());
    if let Some(note) = context.note {
        bundle.set_attribute("note", note.to_owned());
    }

    bundle.set_attribute("org".to_owned(), context.org.to_owned());
    if let Some(project) = context.project {
        bundle.set_attribute("project".to_owned(), project.to_owned());
    }
    if let Some(release) = context.release {
        bundle.set_attribute("release".to_owned(), release.to_owned());
    }
    if let Some(dist) = context.dist {
        bundle.set_attribute("dist".to_owned(), dist.to_owned());
    }

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
        bundle.add_file(bundle_path, file.contents.as_slice(), info)?;
    }

    bundle.finish()?;

    println!(
        "{} Bundled {} {} for upload",
        style(">").dim(),
        style(files.len()).yellow(),
        match files.len() {
            1 => "file",
            _ => "files",
        }
    );

    pb.finish_with_duration("Bundling");

    Ok(archive)
}

fn url_to_bundle_path(url: &str) -> Result<String> {
    let base = Url::parse("http://~").unwrap();
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
        style("> Project:").dim(),
        style(context.project.unwrap_or("None")).yellow()
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
        Some(opts) if opts.supports(ChunkUploadCapability::ArtifactBundles) => "artifact bundle",
        _ => "release bundle",
    };
    println!(
        "{} {}",
        style("> Upload type:").dim(),
        style(upload_type).yellow()
    );
}

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
