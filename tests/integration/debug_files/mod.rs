use crate::integration::register_test;

mod bundle_jvm;
mod upload;

#[test]
fn command_debug_files_help() {
    register_test("debug_files/*.trycmd");
}

#[cfg(not(windows))]
#[test]
fn command_debug_files_not_windows() {
    register_test("debug_files/not_windows/*.trycmd");
}

#[cfg(windows)]
#[test]
fn command_debug_files_windows() {
    register_test("debug_files/windows/*.trycmd");
}
