use std::borrow::Cow;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
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

impl AsRef<[u8]> for DartSymbolMapObject<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl Display for DartSymbolMapObject<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "dartsymbolmap {}", self.name)
    }
}

impl Assemblable for DartSymbolMapObject<'_> {
    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name)
    }

    fn debug_id(&self) -> Option<DebugId> {
        Some(self.debug_id)
    }
}

const MAPPING_ARG: &str = "mapping";
const DEBUG_FILE_ARG: &str = "debug_file";

pub(super) fn make_command(command: Command) -> Command {
    command
        .about("Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.")
        .long_about(
            "Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.{n}{n}Examples:{n}  sentry-cli dart-symbol-map upload --org my-org --project my-proj path/to/dartsymbolmap.json path/to/debug/file{n}{n}The mapping must be a JSON array of strings with an even number of entries (pairs).{n}The debug file must contain exactly one Debug ID. {n}{n}\
    This command is supported on Sentry SaaS and self-hosted versions ≥25.8.0.",
        )
        .arg(
            Arg::new(MAPPING_ARG)
                .value_name("MAPPING")
                .required(true)
                .help("Path to the dartsymbolmap JSON file (e.g. dartsymbolmap.json). Must be a JSON array of strings with an even number of entries (pairs)."),
        )
        .arg(
            Arg::new(DEBUG_FILE_ARG)
                .value_name("DEBUG_FILE")
                .required(true)
                .help("Path to the corresponding debug file to extract the Debug ID from. The file must contain exactly one Debug ID."),
        )
}

pub(super) fn execute(matches: &ArgMatches) -> Result<()> {
    let mapping_path = matches
        .get_one::<String>(MAPPING_ARG)
        .expect("required by clap");
    let debug_file_path = matches
        .get_one::<String>(DEBUG_FILE_ARG)
        .expect("required by clap");

    // Extract Debug ID(s) from the provided debug file
    let dif = DifFile::open_path(debug_file_path, None)?;
    let mut ids: Vec<DebugId> = dif.ids().filter(|id| !id.is_nil()).collect();

    // Ensure a single, unambiguous Debug ID
    ids.sort();
    ids.dedup();
    match ids.len() {
        0 => bail!(
            "No debug identifier found in the provided debug file ({debug_file_path}). Ensure the file contains an embedded Debug ID."
        ),
        1 => {
            let debug_id = ids.remove(0);

            // Validate the dartsymbolmap JSON: must be a JSON array of strings with even length
            let mapping_file_bytes = ByteView::open(mapping_path)
                .with_context(|| format!("Failed to read mapping file at {mapping_path}"))?;
            let mapping_entries: Vec<Cow<'_, str>> =
                serde_json::from_slice(mapping_file_bytes.as_ref())
                    .context("Invalid dartsymbolmap: expected a JSON array of strings")?;

            if !mapping_entries.len().is_multiple_of(2) {
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

            let mapping_len = mapping_file_bytes.len();
            let object = DartSymbolMapObject {
                bytes: mapping_file_bytes.as_ref(),
                name: file_name,
                debug_id,
            };

            // Prepare chunked upload
            let api = Api::current();
            let config = Config::current();
            let org = config.get_org(matches)?;
            let project = config.get_project(matches)?;
            let chunk_upload_options = api
                .authenticated()?
                .get_chunk_upload_options(&org)?;

            // Early file size check against server or default limits (same as debug files)
            let effective_max_file_size = if chunk_upload_options.max_file_size > 0 {
                chunk_upload_options.max_file_size
            } else {
                DEFAULT_MAX_DIF_SIZE
            };

            if (mapping_len as u64) > effective_max_file_size {
                bail!(
                    "The dartsymbolmap '{mapping_path}' exceeds the maximum allowed size ({mapping_len} bytes > {effective_max_file_size} bytes)."
                );
            }

            let options = ChunkOptions::new(chunk_upload_options, &org, &project)
                .with_max_wait(DEFAULT_MAX_WAIT);

            let chunked = Chunked::from(object, options.server_options().chunk_size);
            let (_uploaded, has_processing_errors) = upload_chunked_objects(&[chunked], options)?;
            if has_processing_errors {
                bail!("Some symbol maps did not process correctly");
            }

            Ok(())
        }
        _ => bail!(
            "Multiple debug identifiers found in the provided debug file ({debug_file_path}): {}. Please provide a file that contains a single Debug ID.",
            ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
        ),
    }
}
