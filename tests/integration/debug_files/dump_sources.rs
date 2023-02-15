use crate::integration::register_test;

#[test]
fn command_debug_files_dump_sources() {
    register_test("debug_files/debug_files-dump_sources-dll-embedded-ppdb-with-sources.trycmd");
}
