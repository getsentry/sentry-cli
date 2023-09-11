use crate::integration::register_test;

#[test]
fn command_bash_hook_help() {
    register_test("bash_hook/bash_hook-help.trycmd");
}

#[test]
fn command_bash_hook() {
    register_test("bash_hook/bash_hook.trycmd");
}

#[test]
fn command_bash_hook_tags() {
    register_test("bash_hook/bash_hook-tags.trycmd");
}

#[test]
fn command_bash_hook_release() {
    register_test("bash_hook/bash_hook-release.trycmd");
}

#[test]
fn command_bash_hook_release_using_environment() {
    register_test("bash_hook/bash_hook-release-env.trycmd");
}
