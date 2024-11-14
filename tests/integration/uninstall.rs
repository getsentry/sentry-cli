use crate::integration::TestManager;

#[test]
fn command_uninstall_help() {
    TestManager::new().register_trycmd_test("uninstall/uninstall-help.trycmd");
}

#[test]
fn command_uninstall() {
    #[cfg(not(windows))]
    TestManager::new().register_trycmd_test("uninstall/uninstall.trycmd");
    #[cfg(windows)]
    TestManager::new().register_trycmd_test("uninstall/uninstall-windows.trycmd");
}
