use crate::integration::register_test;

#[test]
fn command_sourcemaps_resolve_help() {
    register_test("sourcemaps/sourcemaps-resolve-help.trycmd");
}

#[test]
fn command_sourcemaps_resolve() {
    register_test("sourcemaps/sourcemaps-resolve.trycmd");
}
