use crate::integration::{register_test_with_opts, AuthMode, RegisterOptions};

#[test]
fn command_send_envelope_help() {
    register_test_with_opts(
        "send_envelope/send_envelope-help.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_send_envelope_no_file() {
    register_test_with_opts(
        "send_envelope/send_envelope-no-file.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_send_envelope_file() {
    register_test_with_opts(
        "send_envelope/send_envelope-file.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_send_envelope_with_logging() {
    register_test_with_opts(
        "send_envelope/send_envelope-file-log.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}
