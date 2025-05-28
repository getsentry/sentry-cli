use crate::integration::TestManager;

#[test]
fn command_mobile_app_upload_help() {
    TestManager::new().register_trycmd_test("mobile_app/mobile_app-upload-help.trycmd");
}
