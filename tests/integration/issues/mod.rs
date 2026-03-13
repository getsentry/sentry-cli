use crate::integration::TestManager;

mod events;
mod list;

#[test]
fn command_issues_help() {
    TestManager::new().register_trycmd_test("issues/issues-help.trycmd");
}
