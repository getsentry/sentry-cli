use crate::integration::register_test;

mod delete;
mod finalize;
mod info;
mod list;
mod new;

#[test]
fn command_releases_no_subcommand() {
    #[cfg(not(windows))]
    register_test("releases/releases-no-subcommand.trycmd");
    #[cfg(windows)]
    register_test("releases/releases-no-subcommand-windows.trycmd");
}
