use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, ChunkUploadCapability};
use crate::config::Config;
use crate::constants::{DEFAULT_MAX_DIF_SIZE, DEFAULT_MAX_WAIT};
use crate::utils::args::ArgExt as _;
use crate::utils::chunks::{upload_chunked_objects, Assemblable, ChunkOptions, Chunked};
use crate::utils::dif::DifFile;
use symbolic::common::DebugId;

struct DartSymbolMapObject {
    bytes: Vec<u8>,
    name: String,
    debug_id: DebugId,
}

impl AsRef<[u8]> for DartSymbolMapObject {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Display for DartSymbolMapObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "dartsymbolmap {}", self.name)
    }
}

impl Assemblable for DartSymbolMapObject {
    fn name(&self) -> Cow<'_, str> {
        Cow::from(self.name.as_str())
    }

    fn debug_id(&self) -> Option<DebugId> {
        Some(self.debug_id)
    }
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.")
        .after_help(
            "Examples:\n  sentry-cli dart-symbol-map upload --org my-org --project my-proj path/to/dartsymbolmap.json path/to/debug/file\n\n  The mapping must be a JSON array of strings with an even number of entries (pairs).\n  The debug file must contain exactly one Debug ID.",
        )
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("mapping")
                .value_name("MAPPING")
                .help("Path to the dartsymbolmap JSON file (e.g. dartsymbolmap.json). Must be a JSON array of strings with an even number of entries (pairs).")
                .required(true),
        )
        .arg(
            Arg::new("debug_file")
                .value_name("DEBUG_FILE")
                .help("Path to the corresponding debug file to extract the Debug ID from. The file must contain exactly one Debug ID.")
                .required(true),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    // Parse required positional arguments
    let mapping_path = matches
        .get_one::<String>("mapping")
        .expect("required argument 'mapping' not provided by clap");
    let debug_file_path = matches
        .get_one::<String>("debug_file")
        .expect("required argument 'debug_file' not provided by clap");

    // Extract Debug ID(s) from the provided debug file
    let dif = DifFile::open_path(debug_file_path, None)?;
    let mut ids: Vec<_> = dif.ids().into_iter().filter(|id| !id.is_nil()).collect();

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
            let mapping_file_bytes = std::fs::read(mapping_path)
                .with_context(|| format!("Failed to read mapping file at {mapping_path}"))?;
            let mapping_entries: Vec<String> = serde_json::from_slice(&mapping_file_bytes)
                .context("Invalid dartsymbolmap: expected a JSON array of strings")?;

            if mapping_entries.len() % 2 != 0 {
                bail!(
                    "Invalid dartsymbolmap: expected an even number of entries (pairs), got {}",
                    mapping_entries.len()
                );
            }

            // Prepare upload object
            let file_name = Path::new(mapping_path)
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or(mapping_path)
                .to_owned();

            let mapping_len = mapping_file_bytes.len();
            let object = DartSymbolMapObject {
                bytes: mapping_file_bytes,
                name: file_name.clone(),
                debug_id,
            };

            // Prepare chunked upload
            let api = Api::current();
            let (org, project) = Config::current().get_org_and_project(matches)?;
            let chunk_upload_options = api
                .authenticated()?
                .get_chunk_upload_options(&org)?
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

            let options = ChunkOptions::new(chunk_upload_options, &org, &project)
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
