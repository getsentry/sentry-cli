use crate::integration::{test_utils::AssertCommand, TestManager};

#[test]
fn command_mobile_app_upload_help() {
    TestManager::new().register_trycmd_test("mobile_app/mobile_app-upload-help.trycmd");
}

#[test]
fn command_mobile_app_upload_invalid_aab() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_aab.aab",
        ])
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_mobile_app_upload_invalid_apk() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_apk.apk",
        ])
        .run_and_assert(AssertCommand::Failure);
}

#[test]
fn command_mobile_app_upload_invalid_xcarchive() {
    TestManager::new()
        .assert_cmd(vec![
            "mobile-app",
            "upload",
            "tests/integration/_fixtures/mobile_app/invalid_xcarchive",
        ])
        .run_and_assert(AssertCommand::Failure);
}
