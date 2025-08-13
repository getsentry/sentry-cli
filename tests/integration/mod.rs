mod bash_hook;
mod build;
mod debug_files;
mod deploys;
mod events;
mod help;
mod info;
mod invalid_env;
mod issues;
mod login;
mod logs;
mod monitors;
mod org_tokens;
mod organizations;
mod projects;
#[cfg(target_os = "macos")]
mod react_native;
mod releases;
mod send_envelope;
mod send_event;
mod sourcemaps;
mod test_utils;
mod token_validation;
mod uninstall;
mod update;
mod upload_dart_symbol_map;
mod upload_dif;
mod upload_dsym;
mod upload_proguard;

use std::fs;
use std::io;
use std::path::Path;

use test_utils::MockEndpointBuilder;
use test_utils::{chunk_upload, env, AssertCommand, TestManager};

pub const UTC_DATE_FORMAT: &str = r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{6,9}Z";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Copy files from source to destination recursively.
pub fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[test]
pub fn token_redacted() {
    TestManager::new().register_trycmd_test("token-redacted.trycmd");
}

#[test]
pub fn token_redacted_2() {
    TestManager::new().register_trycmd_test("token-redacted-2.trycmd");
}
