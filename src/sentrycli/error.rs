use std::error;
use std::process;
use std::fmt;
use std::io;

use std::io::{Read, Write};
use std::string::FromUtf8Error;

use ini::ini;
use clap;
use hyper;
use serde_json;
use url;
use walkdir;
use zip;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub struct CliError {
    repr: CliErrorRepr,
}

#[derive(Debug)]
enum CliErrorRepr {
    ClapError(clap::Error),
    BasicError(String),
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
basic_error!(hyper::error::Error, "could not perform HTTP request");
basic_error!(serde_json::Error, "failed to parse JSON");
basic_error!(FromUtf8Error, "invalid UTF-8");
basic_error!(ini::Error, "ini error");

impl From<hyper::client::response::Response> for CliError {
    fn from(mut resp: hyper::client::response::Response) -> CliError {
        let mut err = String::new();
        resp.read_to_string(&mut err).ok();
        CliError {
            repr: CliErrorRepr::BasicError(format!(
                "request failed ({}, {})", resp.status, err))
        }
    }
}

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
