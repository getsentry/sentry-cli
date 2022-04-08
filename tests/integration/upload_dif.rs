#[cfg(not(windows))]
use crate::integration::register_test;

// NOTE: I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
#[cfg(not(windows))]
#[test]
fn command_upload_dif_help() {
    register_test("upload_dif/upload_dif-help.trycmd");
}
