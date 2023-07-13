use crate::integration::register_test;

#[test]
fn command_debug_files_print_sources_ppdb() {
    register_test("debug_files/debug_files-print_sources-ppdb.trycmd");
}

#[test]
fn command_debug_files_print_sources_dll_embedded_ppdb_with_sources() {
    register_test("debug_files/debug_files-print_sources-dll-embedded-ppdb-with-sources.trycmd");
}

#[test]
fn command_debug_files_print_sources_mixed_embedded_sources() {
    register_test("debug_files/debug_files-print_sources-mixed-embedded-sources.trycmd");
}
