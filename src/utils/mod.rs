//! Various utility functionality.
mod android;
mod args;
mod codepush;
mod enc;
mod formatting;
mod fs;
mod iter;
mod logging;
mod releases;
mod sourcemaps;
mod system;
mod ui;
mod update;
pub mod batch;
pub mod cordova;
pub mod dif;
pub mod upload;
pub mod vcs;
pub mod xcode;

pub use self::android::{dump_proguard_uuids_as_properties, AndroidManifest};
pub use self::args::{get_timestamp, validate_project, validate_seconds, validate_timestamp,
                     validate_uuid, ArgExt};
pub use self::codepush::{get_codepush_package, get_react_native_codepush_release};
pub use self::enc::decode_unknown_string;
pub use self::formatting::{HumanDuration, Table, TableRow};
pub use self::fs::{is_writable, is_zip_file, set_executable_mode, SeekRead, TempDir, TempFile,
                   get_sha1_checksum, get_sha1_checksums};
pub use self::iter::invert_result;
pub use self::logging::Logger;
pub use self::releases::detect_release_name;
pub use self::sourcemaps::{get_sourcemap_reference_from_headers, SourceMapProcessor};
pub use self::system::{expand_envvars, expand_vars, get_family, get_model, init_backtrace,
                       is_homebrew_install, is_npm_install, print_error, propagate_exit_status,
                       run_or_interrupt, to_timestamp};
pub use self::ui::{capitalize_string, copy_with_progress, make_byte_progress_bar, prompt,
                   prompt_to_continue};
pub use self::update::{can_update_sentrycli, get_latest_sentrycli_release,
                       run_sentrycli_update_nagger, SentryCliUpdateInfo};
