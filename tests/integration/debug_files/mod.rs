use crate::integration::TestManager;

mod bundle_jvm;
mod upload;

#[test]
fn command_debug_files_help() {
    TestManager::new().register_trycmd_test("debug_files/*.trycmd");
}

#[cfg(not(windows))]
#[test]
fn command_debug_files_not_windows() {
    TestManager::new().register_trycmd_test("debug_files/not_windows/*.trycmd");
}

#[cfg(windows)]
#[test]
fn command_debug_files_windows() {
    TestManager::new().register_trycmd_test("debug_files/windows/*.trycmd");
}
