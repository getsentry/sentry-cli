use std::error;
use std::process;
use std::fmt;
use std::io;

use std::io::Write;

use clap;
use hyper;
use serde_json;
use url;
use walkdir;
use zip;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub struct CliError {
    repr: ErrorRepr,
}

#[derive(Debug)]
enum ErrorRepr {
    ClapError(clap::Error),
    BasicError(String),
    IoError(io::Error),
}

macro_rules! wrap_error {
    ($ty:ty, $wrapper:expr) => {
        impl From<$ty> for CliError {
            fn from(err: $ty) -> CliError {
                CliError {
                    repr: $wrapper(err)
                }
            }
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

wrap_error!(io::Error, ErrorRepr::IoError);
wrap_error!(clap::Error, ErrorRepr::ClapError);
basic_error!(zip::result::ZipError, "could not zip");
basic_error!(walkdir::Error, "could not walk path");
basic_error!(url::ParseError, "could not parse URL");
basic_error!(hyper::error::Error, "could not perform HTTP request");
basic_error!(serde_json::Error, "failed to parse JSON");

impl From<String> for CliError {
    fn from(err: String) -> CliError {
        CliError {
            repr: ErrorRepr::BasicError(err)
        }
    }
}

impl<'a> From<&'a str> for CliError {
    fn from(err: &'a str) -> CliError {
        CliError {
            repr: ErrorRepr::BasicError(err.to_owned())
        }
    }
}

impl CliError {

    pub fn exit(&self) -> ! {
        match self.repr {
            ErrorRepr::ClapError(ref err) => err.exit(),
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
            ErrorRepr::BasicError(ref msg) => write!(f, "{}", msg),
            ErrorRepr::IoError(ref err) => write!(f, "i/o failure: {}", err),
            ErrorRepr::ClapError(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for CliError {
    fn description(&self) -> &str {
        match self.repr {
            ErrorRepr::BasicError(ref msg) => &msg,
            ErrorRepr::IoError(ref err) => err.description(),
            ErrorRepr::ClapError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.repr {
            ErrorRepr::IoError(ref err) => Some(&*err),
            ErrorRepr::ClapError(ref err) => Some(&*err),
            _ => None,
        }
    }
}
