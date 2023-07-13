use crate::integration::register_test;

#[test]
fn command_debug_files_check() {
    register_test("debug_files/debug_files-check.trycmd");
}

#[test]
fn command_debug_files_check_no_file() {
    #[cfg(not(windows))]
    register_test("debug_files/debug_files-check-no-file.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-check-no-file-windows.trycmd");
}

#[test]
fn command_debug_files_check_no_file_allow_failure() {
    #[cfg(not(windows))]
    register_test("debug_files/debug_files-check-no-file-allow-failure.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-check-no-file-allow-failure-windows.trycmd");
}

#[test]
fn command_debug_files_check_no_file_allow_failure_env() {
    #[cfg(not(windows))]
    register_test("debug_files/debug_files-check-no-file-allow-failure-env.trycmd");
    #[cfg(windows)]
    register_test("debug_files/debug_files-check-no-file-allow-failure-env-windows.trycmd");
}

#[test]
fn command_debug_files_check_dll_embedded_ppdb_with_sources() {
    register_test("debug_files/debug_files-check-dll-embedded-ppdb-with-sources.trycmd");
}
