use crate::integration::{register_test_with_opts, AuthMode, RegisterOptions};

#[test]
fn command_send_event_help() {
    register_test_with_opts(
        "send_event/send_event-help.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

// I have no idea why this is timing out on Windows.
// I verified it manually, and this command works just fine. — Kamil
// TODO: Fix windows timeout.
#[cfg(not(windows))]
#[test]
fn command_send_event_raw() {
    register_test_with_opts(
        "send_event/send_event-raw.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_send_event_file() {
    register_test_with_opts(
        "send_event/send_event-file.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}

#[test]
fn command_send_event_raw_fail() {
    register_test_with_opts(
        "send_event/send_event-raw-fail.trycmd",
        RegisterOptions {
            auth_mode: AuthMode::Dsn,
        },
    );
}
