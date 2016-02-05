use std::error;
use std::fmt;
use std::io;

use hyper;
use url;
use walkdir;
use zip;

#[derive(Debug)]
pub struct CliError {
    repr: ErrorRepr,
}

#[derive(Debug)]
enum ErrorRepr {
    BasicError(String),
    IoError(io::Error),
    Abort(i32),
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError {
            repr: ErrorRepr::IoError(err),
        }
    }
}

macro_rules! basic_error {
    ($ty:ty, $msg:expr) => {
        impl From<$ty> for CliError {
            fn from(err: $ty) -> CliError {
                CliError {
                    repr: ErrorRepr::BasicError(format!("{}: {}", $msg, err))
                }
            }
        }
    }
}

basic_error!(zip::result::ZipError, "could not zip");
basic_error!(walkdir::Error, "could not walk path");
basic_error!(url::ParseError, "could not parse url");
basic_error!(hyper::error::Error, "could not perform http request");

impl CliError {

    pub fn abort_with_exit_code(code: i32) -> CliError {
        CliError {
            repr: ErrorRepr::Abort(code),
        }
    }

    pub fn unknown_command(msg: &str) -> CliError {
        CliError {
            repr: ErrorRepr::BasicError(format!("unknown command '{}'", msg)),
        }
    }

    pub fn is_silent(&self) -> bool {
        match self.repr {
            ErrorRepr::Abort(..) => true,
            _ => false
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.repr {
            ErrorRepr::Abort(code) => code,
            _ => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.repr {
            ErrorRepr::Abort(..) => Ok(()),
            ErrorRepr::BasicError(ref msg) => write!(f, "{}", msg),
            ErrorRepr::IoError(ref err) => write!(f, "i/o failure: {}", err),
        }
    }
}

impl error::Error for CliError {
    fn description(&self) -> &str {
        match self.repr {
            ErrorRepr::Abort(code) => {
                if code == 0 {
                    "abort with success code"
                } else {
                    "abort with error code"
                }
            },
            ErrorRepr::BasicError(ref msg) => &msg,
            ErrorRepr::IoError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.repr {
            ErrorRepr::IoError(ref err) => Some(&*err),
            _ => None,
        }
    }
}
