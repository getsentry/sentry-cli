//! Various utility functionality.
mod android;
mod args;
mod codepush;
mod formatting;
mod fs;
mod logging;
mod macho;
mod releases;
mod sourcemaps;
mod system;
mod ui;
mod update;
pub mod dif;
pub mod vcs;
pub mod xcode;

pub use self::android::{AndroidManifest, dump_proguard_uuids_as_properties};
pub use self::args::{ArgExt, validate_uuid, validate_seconds, validate_timestamp,
                     validate_project, get_timestamp};
pub use self::codepush::{get_codepush_package, get_codepush_release};
pub use self::formatting::{HumanDuration, Table, TableRow};
pub use self::fs::{TempFile, is_writable, set_executable_mode, is_zip_file,
                   get_sha1_checksum};
pub use self::logging::Logger;
pub use self::macho::MachoInfo;
pub use self::releases::detect_release_name;
pub use self::sourcemaps::{SourceMapProcessor, get_sourcemap_reference_from_headers};
pub use self::system::{propagate_exit_status, is_homebrew_install,
                       is_npm_install, expand_envvars, expand_vars,
                       print_error, to_timestamp,
                       run_or_interrupt, init_backtrace};
pub use self::ui::{prompt_to_continue, prompt, capitalize_string,
                   copy_with_progress, make_byte_progress_bar};
pub use self::update::{can_update_sentrycli, get_latest_sentrycli_release,
                       SentryCliUpdateInfo};
