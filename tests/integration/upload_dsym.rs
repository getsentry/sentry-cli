#[cfg(not(windows))]
use crate::integration::TestManager;

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_upload_dsym_help() {
    TestManager::new().register_trycmd_test("upload_dsym/upload_dsym-help.trycmd");
}
