//! Implements the generic error type for the library.
use std::error;
use std::process;
use std::fmt;
use std::io;
use std::io::Write;
use std::string::FromUtf8Error;

use ini::ini;
use clap;
use serde_json;
use url;
use walkdir;
use zip;
use sourcemap;

use api;

/// Common result type.
pub type CliResult<T> = Result<T, CliError>;

/// The error type for this library.
#[derive(Debug)]
pub struct CliError {
    repr: CliErrorRepr,
}

#[derive(Debug)]
enum CliErrorRepr {
    ClapError(clap::Error),
    BasicError(String),
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    detail: Option<String>,
    error: Option<String>,
}

macro_rules! basic_error {
    ($ty:ty, $msg:expr) => {
        impl From<$ty> for CliError {
            fn from(err: $ty) -> CliError {
                CliError {
                    repr: CliErrorRepr::BasicError(format!("{}: {}", $msg, err))
                }
            }
        }
    }
}

basic_error!(io::Error, "i/o failure");
basic_error!(zip::result::ZipError, "could not zip");
basic_error!(walkdir::Error, "could not walk path");
basic_error!(url::ParseError, "could not parse URL");
basic_error!(serde_json::Error, "failed to parse JSON");
basic_error!(FromUtf8Error, "invalid UTF-8");
basic_error!(ini::Error, "ini error");
basic_error!(sourcemap::Error, "sourcemap error");
basic_error!(api::Error, "could not perform API request");

impl From<clap::Error> for CliError {
    fn from(err: clap::Error) -> CliError {
        CliError {
            repr: CliErrorRepr::ClapError(err)
        }
    }
}

impl From<String> for CliError {
    fn from(err: String) -> CliError {
        CliError {
            repr: CliErrorRepr::BasicError(err)
        }
    }
}

impl<'a> From<&'a str> for CliError {
    fn from(err: &'a str) -> CliError {
        CliError {
            repr: CliErrorRepr::BasicError(err.to_owned())
        }
    }
}

impl CliError {

    /// Exists the process and prints out the error if needed.
    pub fn exit(&self) -> ! {
        match self.repr {
            CliErrorRepr::ClapError(ref err) => err.exit(),
            _ => {
                writeln!(&mut io::stderr(), "error: {}", self).ok();
                process::exit(1)
            },
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.repr {
            CliErrorRepr::BasicError(ref msg) => write!(f, "{}", msg),
            CliErrorRepr::ClapError(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for CliError {
    fn description(&self) -> &str {
        match self.repr {
            CliErrorRepr::BasicError(ref msg) => &msg,
            CliErrorRepr::ClapError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.repr {
            CliErrorRepr::ClapError(ref err) => Some(&*err),
            _ => None,
        }
    }
}
