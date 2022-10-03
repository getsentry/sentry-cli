use crate::integration::register_test;

#[test]
fn command_debug_files_check() {
    register_test("debug_files/debug_files-check.trycmd");
}

#[test]
fn command_debug_files_check_no_file_allow_failure() {
    register_test("debug_files/debug_files-check-no-file.trycmd");
}
