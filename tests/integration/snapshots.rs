use crate::integration::TestManager;

#[test]
fn command_snapshots_diff_help() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-diff-help.trycmd");
}

#[test]
fn command_snapshots_diff_missing_dir() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-diff-missing-dir.trycmd");
}

#[test]
fn command_snapshots_upload_help() {
    TestManager::new().register_trycmd_test("snapshots/snapshots-upload-help.trycmd");
}
