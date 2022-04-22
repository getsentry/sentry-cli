use crate::integration::register_test;

#[test]
fn command_debug_files_check() {
    register_test("debug_files/debug_files-check.trycmd");
}

#[test]
fn command_debug_files_check_wasm() {
    register_test("debug_files/debug_files-check-wasm.trycmd");
}
