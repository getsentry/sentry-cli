use crate::integration::register_test;

#[test]
fn command_login_help() {
    register_test("login/login-help.trycmd");
}

#[test]
fn command_login() {
    register_test("login/login.trycmd");
}

#[test]
fn command_login_with_token() {
    register_test("login/login-with-token.trycmd");
}