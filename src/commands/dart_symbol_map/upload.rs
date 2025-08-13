use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use clap::Args;

use crate::api::{Api, ChunkUploadCapability};
use crate::config::Config;
use crate::constants::{DEFAULT_MAX_DIF_SIZE, DEFAULT_MAX_WAIT};
use crate::utils::chunks::{upload_chunked_objects, Assemblable, ChunkOptions, Chunked};
use crate::utils::dif::DifFile;
use symbolic::common::ByteView;
use symbolic::common::DebugId;

struct DartSymbolMapObject<'a> {
    bytes: &'a [u8],
    name: &'a str,
    debug_id: DebugId,
}

impl<'a> AsRef<[u8]> for DartSymbolMapObject<'a> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl<'a> Display for DartSymbolMapObject<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "dartsymbolmap {}", self.name)
    }
}

impl<'a> Assemblable for DartSymbolMapObject<'a> {
    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name)
    }

    fn debug_id(&self) -> Option<DebugId> {
        Some(self.debug_id)
    }
}

#[derive(Args, Clone)]
pub(crate) struct DartSymbolMapUploadArgs {
    #[arg(short = 'o', long = "org")]
    #[arg(help = "The organization ID or slug.")]
    pub(super) org: Option<String>,

    #[arg(short = 'p', long = "project")]
    #[arg(help = "The project ID or slug.")]
    pub(super) project: Option<String>,

    #[arg(value_name = "MAPPING")]
    #[arg(
        help = "Path to the dartsymbolmap JSON file (e.g. dartsymbolmap.json). Must be a JSON array of strings with an even number of entries (pairs)."
    )]
    pub(super) mapping: String,

    #[arg(value_name = "DEBUG_FILE")]
    #[arg(
        help = "Path to the corresponding debug file to extract the Debug ID from. The file must contain exactly one Debug ID."
    )]
    pub(super) debug_file: String,
}

pub(super) fn execute(args: DartSymbolMapUploadArgs) -> Result<()> {
    let mapping_path = &args.mapping;
    let debug_file_path = &args.debug_file;

    // Extract Debug ID(s) from the provided debug file
    let dif = DifFile::open_path(debug_file_path, None)?;
    let mut ids: Vec<DebugId> = dif.ids().filter(|id| !id.is_nil()).collect();

    // Ensure a single, unambiguous Debug ID
    ids.sort();
    ids.dedup();
    match ids.len() {
        0 => bail!(
            "No debug identifier found in the provided debug file ({}). Ensure the file contains an embedded Debug ID.",
            debug_file_path
        ),
        1 => {
            let debug_id = ids.remove(0);

            // Validate the dartsymbolmap JSON: must be a JSON array of strings with even length
            let mapping_file_bytes = ByteView::open(mapping_path)
                .with_context(|| format!("Failed to read mapping file at {mapping_path}"))?;
            let mapping_entries: Vec<Cow<'_, str>> =
                serde_json::from_slice(mapping_file_bytes.as_ref())
                    .context("Invalid dartsymbolmap: expected a JSON array of strings")?;

            if mapping_entries.len() % 2 != 0 {
                bail!(
                    "Invalid dartsymbolmap: expected an even number of entries, got {}",
                    mapping_entries.len()
                );
            }

            // Prepare upload object
            let file_name = Path::new(mapping_path)
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or(mapping_path);

            // Prepend a marker.
            //
            // The Sentry backend deduplicates chunked uploads by content checksum. If the same
            // mapping file is uploaded for multiple debug IDs, the raw file contents are identical
            // and the backend rejects subsequent uploads as duplicates. To ensure each upload has a
            // distinct checksum without mutating the user's file on disk, we inject a marker pair at
            // the start of the JSON array: ["SENTRY_DEBUG_ID_MARKER", "<debug_id>"]. This keeps the
            // structure valid (even-length array of strings) while making the payload unique per
            // debug ID.
            let mut modified_entries: Vec<Cow<'_, str>> =
                Vec::with_capacity(mapping_entries.len() + 2);
            modified_entries.push(Cow::Borrowed("SENTRY_DEBUG_ID_MARKER"));
            modified_entries.push(Cow::Owned(debug_id.to_string()));
            modified_entries.extend(mapping_entries.into_iter());

            let modified_mapping_bytes: Vec<u8> = serde_json::to_vec(&modified_entries)
                .context("Failed to serialize modified dartsymbolmap JSON")?;

            let mapping_len = mapping_file_bytes.len();
            let object = DartSymbolMapObject {
                bytes: &modified_mapping_bytes,
                name: file_name,
                debug_id,
            };

            // Prepare chunked upload
            let api = Api::current();
            // Resolve org and project like logs: prefer args, fallback to defaults
            let config = Config::current();
            let (default_org, default_project) = config.get_org_and_project_defaults();
            let org = args
                .org
                .as_ref()
                .or(default_org.as_ref())
                .ok_or_else(|| anyhow::anyhow!(
                    "No organization specified. Please specify an organization using the --org argument."
                ))?;
            let project = args
                .project
                .as_ref()
                .or(default_project.as_ref())
                .ok_or_else(|| anyhow::anyhow!(
                    "No project specified. Use --project or set a default in config."
                ))?;
            let chunk_upload_options = api
                .authenticated()?
                .get_chunk_upload_options(org)?
                .ok_or_else(|| anyhow::anyhow!(
                    "server does not support chunked uploading. Please update your Sentry server."
                ))?;

            if !chunk_upload_options.supports(ChunkUploadCapability::DartSymbolMap) {
                bail!(
                    "Server does not support uploading Dart symbol maps via chunked upload. Please update your Sentry server."
                );
            }

            // Early file size check against server or default limits (same as debug files)
            let effective_max_file_size = if chunk_upload_options.max_file_size > 0 {
                chunk_upload_options.max_file_size
            } else {
                DEFAULT_MAX_DIF_SIZE
            };

            if (mapping_len as u64) > effective_max_file_size {
                bail!(
                    "The dartsymbolmap '{}' exceeds the maximum allowed size ({} bytes > {} bytes).",
                    mapping_path,
                    mapping_len,
                    effective_max_file_size
                );
            }

            let options = ChunkOptions::new(chunk_upload_options, org, project)
                .with_max_wait(DEFAULT_MAX_WAIT);

            let chunked = Chunked::from(object, options.server_options().chunk_size as usize)?;
            let (_uploaded, has_processing_errors) = upload_chunked_objects(&[chunked], options)?;
            if has_processing_errors {
                bail!("Some symbol maps did not process correctly");
            }

            Ok(())
        }
        _ => bail!(
            "Multiple debug identifiers found in the provided debug file ({}): {}. Please provide a file that contains a single Debug ID.",
            debug_file_path,
            ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
        ),
    }
}
