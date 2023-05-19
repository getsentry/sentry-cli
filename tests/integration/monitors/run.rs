use crate::integration::register_test;

#[test]
fn command_monitors_run() {
    if cfg!(windows) {
        register_test("monitors/monitors-run-win.trycmd");
    } else {
        register_test("monitors/monitors-run.trycmd");
    }
}

#[test]
fn command_monitors_run_osenv() {
    register_test("monitors/monitors-run-osenv.trycmd");
}

#[test]
fn command_monitors_run_environment() {
    register_test("monitors/monitors-run-environment.trycmd");
}

#[test]
fn command_monitors_run_help() {
    register_test("monitors/monitors-run-help.trycmd");
}
