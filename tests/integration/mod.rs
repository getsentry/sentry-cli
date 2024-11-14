mod bash_hook;
mod debug_files;
mod deploys;
mod events;
mod help;
mod info;
mod invalid_env;
mod issues;
mod login;
mod monitors;
mod org_tokens;
mod organizations;
mod projects;
#[cfg(target_os = "macos")]
mod react_native;
mod releases;
mod send_envelope;
mod send_event;
mod send_metric;
mod sourcemaps;
mod test_utils;
mod token_validation;
mod uninstall;
mod update;
mod upload_dif;
mod upload_dsym;
mod upload_proguard;

use std::fs;
use std::io;
use std::path::Path;

use mockito::{self, Mock};
use trycmd::TestCases;

use test_utils::env;
use test_utils::{
    mock_common_upload_endpoints, mock_endpoint, ChunkOptions, MockEndpointBuilder, ServerBehavior,
};

pub const UTC_DATE_FORMAT: &str = r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{6,9}Z";
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn register_test_without_token(path: &str) -> TestCases {
    let test_case = TestCases::new();

    env::set(|k, v| {
        test_case.env(k, v);
    });

    test_case.case(format!("tests/integration/_cases/{path}"));
    test_case.insert_var("[VERSION]", VERSION).unwrap();
    test_case
}
pub fn register_test(path: &str) -> TestCases {
    let test_case = register_test_without_token(path);

    env::set_auth_token(|k, v| {
        test_case.env(k, v);
    });

    test_case
}

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

pub fn assert_endpoints(mocks: &[Mock]) {
    for mock in mocks {
        mock.assert();
    }
}

#[test]
pub fn token_redacted() {
    register_test("token-redacted.trycmd");
}

#[test]
pub fn token_redacted_2() {
    register_test("token-redacted-2.trycmd");
}
