/// Indicates that sentry-cli should quit without printing anything.
#[derive(Fail, Debug)]
#[fail(display = "sentry-cli exit with {}", _0)]
pub struct QuietExit(pub i32);
