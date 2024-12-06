use std::collections::BTreeMap;
use std::fmt::Display;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use indicatif::ProgressStyle;

use super::{Assemblable, Chunk, Chunked, MissingObjectsInfo};
use crate::api::{Api, AssembleDifsRequest, ChunkServerOptions, ChunkedFileState, DebugInfoFile};
use crate::utils::progress::ProgressBar;

/// A trait representing options for chunk uploads.
/// The trait also provides the `upload_chunked_objects` method, which
/// executes the upload given the options and a list of chunked objects.
pub trait ChunkOptions: Sized {
    /// Determines whether we need to strip debug_ids from the requests.
    /// When this function returns `true`, the caller is responsible for stripping
    /// the debug_ids from the requests, to maintain backwards compatibility with
    /// older Sentry servers.
    fn should_strip_debug_ids(&self) -> bool;

    /// Returns the organization that we are uploading to.
    fn org(&self) -> &str;

    /// Returns the project that we are uploading to.
    fn project(&self) -> &str;

    /// Returns whether we should wait for assembling to complete.
    fn should_wait(&self) -> bool;

    /// Returns the maximum wait time for the upload to complete.
    fn max_wait(&self) -> Duration;

    /// Returns the server options for the chunk upload.
    fn server_options(&self) -> &ChunkServerOptions;

    /// Uploads the given chunked objects to the server.
    fn upload_chunked_objects<T>(self, chunked: &[Chunked<T>]) -> Result<(Vec<DebugInfoFile>, bool)>
    where
        T: AsRef<[u8]> + Assemblable + Display,
    {
        // Upload missing chunks to the server and remember incomplete objects
        let missing_info = try_assemble(chunked, &self)?;
        upload_missing_chunks(&missing_info, self.server_options())?;

        // Only if objects were missing, poll until assembling is complete
        let (missing_objects, _) = missing_info;
        if !missing_objects.is_empty() {
            poll_assemble(&missing_objects, &self)
        } else {
            println!(
                "{} Nothing to upload, all files are on the server",
                console::style(">").dim()
            );

            Ok((Default::default(), false))
        }
    }
}

/// Calls the assemble endpoint and returns the state for every object along
/// with info on missing chunks.
///
/// The returned value contains separate vectors for incomplete objects and
/// missing chunks for convenience.
fn try_assemble<'m, T>(
    objects: &'m [Chunked<T>],
    options: &impl ChunkOptions,
) -> Result<MissingObjectsInfo<'m, T>>
where
    T: AsRef<[u8]> + Assemblable,
{
    let api = Api::current();
    let mut request: AssembleDifsRequest<'_> = objects.iter().collect();

    if options.should_strip_debug_ids() {
        request.strip_debug_ids();
    }

    let response =
        api.authenticated()?
            .assemble_difs(options.org(), options.project(), &request)?;

    // We map all objects by their checksum, so we can access them faster when
    // iterating through the server response below. Since the caller will invoke
    // this function multiple times (most likely twice), this operation is
    // performed twice with the same data. While this is redundant, it is also
    // fast enough and keeping it here makes the `try_assemble` interface
    // nicer.
    let objects_by_checksum = objects
        .iter()
        .map(|m| (m.checksum(), m))
        .collect::<BTreeMap<_, _>>();

    let mut objects = Vec::new();
    let mut chunks = Vec::new();
    for (checksum, ref file_response) in response {
        let chunked_match = *objects_by_checksum
            .get(&checksum)
            .ok_or_else(|| anyhow::anyhow!("Server returned unexpected checksum"))?;

        match file_response.state {
            ChunkedFileState::Error => {
                // One of the files could not be uploaded properly and resulted
                // in an error. We include this file in the return value so that
                // it shows up in the final report.
                objects.push(chunked_match);
            }
            ChunkedFileState::Assembling => {
                // This file is currently assembling. The caller will have to poll this file later
                // until it either resolves or errors.
                objects.push(chunked_match);
            }
            ChunkedFileState::NotFound => {
                // Assembling for one of the files has not started because some
                // (or all) of its chunks have not been found. We report its
                // missing chunks to the caller and then continue. The caller
                // will have to call `try_assemble` again after uploading
                // them.
                let mut missing_chunks = chunked_match
                    .iter_chunks()
                    .filter(|&Chunk((c, _))| file_response.missing_chunks.contains(&c))
                    .peekable();

                // Usually every file that is NotFound should also contain a set
                // of missing chunks. However, if we tried to upload an empty
                // file or the server returns an invalid response, we need to
                // make sure that this match is not included in the missing
                // objects.
                if missing_chunks.peek().is_some() {
                    objects.push(chunked_match);
                }

                chunks.extend(missing_chunks);
            }
            _ => {
                // This file has already finished. No action required anymore.
            }
        }
    }
    Ok((objects, chunks))
}

/// Concurrently uploads chunks specified in `missing_info` in batches. The
/// batch size and number of concurrent requests is controlled by
/// `chunk_options`.
///
/// This function blocks until all chunks have been uploaded.
fn upload_missing_chunks<T>(
    missing_info: &MissingObjectsInfo<'_, T>,
    chunk_options: &ChunkServerOptions,
) -> Result<()> {
    let (objects, chunks) = missing_info;

    // Chunks might be empty if errors occurred in a previous upload. We do
    // not need to render a progress bar or perform an upload in this case.
    if chunks.is_empty() {
        return Ok(());
    }

    let progress_style = ProgressStyle::default_bar().template(&format!(
        "{} Uploading {} missing debug information file{}...\
         \n{{wide_bar}}  {{bytes}}/{{total_bytes}} ({{eta}})",
        console::style(">").dim(),
        console::style(objects.len().to_string()).yellow(),
        if objects.len() == 1 { "" } else { "s" }
    ));

    super::upload_chunks(chunks, chunk_options, progress_style)?;

    println!(
        "{} Uploaded {} missing debug information {}",
        console::style(">").dim(),
        console::style(objects.len().to_string()).yellow(),
        match objects.len() {
            1 => "file",
            _ => "files",
        }
    );

    Ok(())
}

/// Polls the assemble endpoint until all objects have either completed or errored. Returns a list of
/// `DebugInfoFile`s that have been created successfully and also prints a summary to the user.
///
/// This function assumes that all chunks have been uploaded successfully. If there are still
/// missing chunks in the assemble response, this likely indicates a bug in the server.
fn poll_assemble<T>(
    chunked_objects: &[&Chunked<T>],
    options: &impl ChunkOptions,
) -> Result<(Vec<DebugInfoFile>, bool)>
where
    T: Display + Assemblable,
{
    let progress_style = ProgressStyle::default_bar().template(
        "{prefix:.dim} Processing files...\
         \n{wide_bar}  {pos}/{len}",
    );

    let api = Api::current();
    let pb = ProgressBar::new(chunked_objects.len());
    pb.set_style(progress_style);
    pb.set_prefix(">");

    let assemble_start = Instant::now();

    let mut request: AssembleDifsRequest<'_> = chunked_objects.iter().copied().collect();
    if options.should_strip_debug_ids() {
        request.strip_debug_ids();
    }

    let response = loop {
        let response =
            api.authenticated()?
                .assemble_difs(options.org(), options.project(), &request)?;

        let chunks_missing = response
            .values()
            .any(|r| r.state == ChunkedFileState::NotFound);

        if chunks_missing {
            return Err(anyhow::anyhow!(
                "Some uploaded files are now missing on the server. Please retry by running \
                 `sentry-cli upload-dif` again. If this problem persists, please report a bug.",
            ));
        }

        // Poll until there is a response, unless the user has specified to skip polling. In
        // that case, we return the potentially partial response from the server. This might
        // still contain a cached error.
        if !options.should_wait() {
            break response;
        }

        if assemble_start.elapsed() > options.max_wait() {
            break response;
        }

        let pending = response
            .iter()
            .filter(|&(_, r)| r.state.is_pending())
            .count();

        pb.set_position((chunked_objects.len() - pending) as u64);

        if pending == 0 {
            break response;
        }

        thread::sleep(super::ASSEMBLE_POLL_INTERVAL);
    };

    pb.finish_and_clear();
    if response.values().any(|r| r.state.is_pending()) {
        println!("{} File upload complete:\n", console::style(">").dim());
    } else {
        println!("{} File processing complete:\n", console::style(">").dim());
    }

    let (errors, mut successes): (Vec<_>, _) = response
        .into_iter()
        .partition(|(_, r)| r.state.is_err() || options.should_wait() && r.state.is_pending());

    // Print a summary of all successes first, so that errors show up at the
    // bottom for the user
    successes.sort_by_key(|(_, success)| {
        success
            .dif
            .as_ref()
            .map(|x| x.object_name.as_str())
            .unwrap_or("")
            .to_owned()
    });

    let objects_by_checksum: BTreeMap<_, _> =
        chunked_objects.iter().map(|m| (m.checksum(), m)).collect();

    for &(checksum, ref success) in &successes {
        // Silently skip all OK entries without a "dif" record since the server
        // will always return one.
        if let Some(ref dif) = success.dif {
            // Files that have completed processing will contain a `dif` record
            // returned by the server. Use this to show detailed information.
            println!(
                "  {:>7} {} ({}; {}{})",
                console::style("OK").green(),
                console::style(&dif.id()).dim(),
                dif.object_name,
                dif.cpu_name,
                dif.data.kind.map(|c| format!(" {c:#}")).unwrap_or_default()
            );

            render_detail(&success.detail, None);
        } else if let Some(object) = objects_by_checksum.get(&checksum) {
            // If we skip waiting for the server to finish processing, there
            // are pending entries. We only expect results that have been
            // uploaded in the first place, so we can skip everything else.
            println!("  {:>8} {}", console::style("UPLOADED").yellow(), object);
        }
        // All other entries will be in the `errors` list.
    }

    // Print a summary of all errors at the bottom.
    let mut errored = vec![];
    for (checksum, error) in errors {
        let object = objects_by_checksum
            .get(&checksum)
            .ok_or_else(|| anyhow::anyhow!("Server returned unexpected checksum"))?;
        errored.push((object, error));
    }
    errored.sort_by_key(|x| x.0.name());

    let has_errors = !errored.is_empty();
    for (object, error) in errored {
        let fallback = match error.state {
            ChunkedFileState::Assembling => Some("The file is still processing and not ready yet"),
            ChunkedFileState::NotFound => Some("The file could not be saved"),
            _ => Some("An unknown error occurred"),
        };

        println!("  {:>7} {}", console::style("ERROR").red(), object.name());
        render_detail(&error.detail, fallback);
    }

    // Return only successful uploads
    Ok((
        successes.into_iter().filter_map(|(_, r)| r.dif).collect(),
        has_errors,
    ))
}

/// Renders the given detail string to the command line. If the `detail` is
/// either missing or empty, the optional fallback will be used.
fn render_detail(detail: &Option<String>, fallback: Option<&str>) {
    let mut string = match *detail {
        Some(ref string) => string.as_str(),
        None => "",
    };

    if string.is_empty() {
        if let Some(fallback) = fallback {
            string = fallback;
        }
    }

    for line in string.lines() {
        if !line.is_empty() {
            println!("        {}", console::style(line).dim());
        }
    }
}
