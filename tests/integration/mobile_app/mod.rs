#![cfg(feature = "unstable-mobile-app")]

use crate::integration::TestManager;

mod upload;

#[test]
fn command_mobile_app_help() {
    TestManager::new().register_trycmd_test("mobile_app/mobile_app-help.trycmd");
}
