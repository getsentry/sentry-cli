use crate::integration::TestManager;

#[test]
fn command_code_mappings_help() {
    TestManager::new().register_trycmd_test("code_mappings/code-mappings-help.trycmd");
}

#[test]
fn command_code_mappings_no_subcommand() {
    TestManager::new().register_trycmd_test("code_mappings/code-mappings-no-subcommand.trycmd");
}

#[test]
fn command_code_mappings_upload_help() {
    TestManager::new().register_trycmd_test("code_mappings/code-mappings-upload-help.trycmd");
}
