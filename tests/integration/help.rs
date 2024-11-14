use crate::integration::TestManager;

#[test]
fn command_help() {
    let manager = TestManager::new();

    #[cfg(not(windows))]
    manager.register_trycmd_test("help/help.trycmd");
    #[cfg(windows)]
    manager.register_trycmd_test("help/help-windows.trycmd");
}
