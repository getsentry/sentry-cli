use std::io;
use std::env;
use std::process;

use argparse;
use url::Url;
use hyper::client::request::Request;
use hyper::net::Fresh;
use hyper::method::Method;
use hyper::header::{Authorization, Basic};

use super::{CliResult, CliError};

#[derive(Debug)]
pub struct Config {
    token: String,
    url: String,
    verbose: bool,
}

impl Config {

    pub fn api_request(&self, method: Method, path: &str) -> CliResult<Request<Fresh>> {
        let url = try!(Url::parse(&format!("{}/api/0{}", self.url.trim_right_matches("/"), path)));
        println!("{}", url);
        let mut request = try!(Request::new(method, url));
        {
            let mut headers = request.headers_mut();
            headers.set(Authorization(Basic {
                username: self.token.clone(),
                password: None
            }));
        }
        Ok(request)
    }
}

macro_rules! each_subcommand {
    ($mac:ident) => ({
        $mac!(upload_dsym);
    })
}

pub fn parse_args_or_abort(ap: &argparse::ArgumentParser, args: Vec<String>) -> CliResult<()> {
    match ap.parse(args, &mut io::stdout(), &mut io::stderr()) {
        Ok(()) => Ok(()),
        Err(code) => Err(CliError::abort_with_exit_code(code)),
    }
}

pub fn execute(args: Vec<String>, config: &mut Config) -> CliResult<()> {
    let prog_name = args[0].clone();
    let mut show_version = false;
    let mut subcommand = "".to_owned();
    let mut subcommand_args : Vec<String> = vec![];

    {
        let mut ap = argparse::ArgumentParser::new();
        ap.set_description("Sentry command line utility.");
        ap.refer(&mut show_version)
            .add_option(&["---version"], argparse::StoreTrue,
                        "print the version and exit");
        ap.refer(&mut config.verbose)
            .add_option(&["-v", "--verbose"], argparse::StoreTrue,
                        "enable verbose mode");
        ap.refer(&mut config.url)
            .add_option(&["--url"], argparse::Store,
                        "the sentry API url");
        ap.refer(&mut config.token)
            .add_option(&["--token"], argparse::Store,
                        "the sentry api token to use");
        ap.refer(&mut subcommand)
            .required()
            .add_argument("command", argparse::Store,
                          "the command to run");
        ap.refer(&mut subcommand_args)
            .add_argument("arguments", argparse::List,
                          "arguments for the subcommand");
        ap.stop_on_first_argument(true);
        try!(parse_args_or_abort(&ap, args));
    }

    if show_version {
        println!("sentry-cli {}", super::get_version());
        return Ok(());
    }

    macro_rules! cmd {
        ($name:ident) => {
            if subcommand == stringify!($name).replace("_", "-") {
                mod $name;
                subcommand_args.insert(0, format!("{} {}", prog_name, subcommand));
                return $name::execute(subcommand_args, &config);
            }
        }
    }

    each_subcommand!(cmd);
    Err(CliError::unknown_command(&subcommand))
}

pub fn run() -> CliResult<()> {
    let mut cfg = Config {
        token: env::var("SENTRY_TOKEN").unwrap_or("".to_owned()),
        url: "https://api.getsentry.com/".to_owned(),
        verbose: false,
    };
    execute(env::args().collect(), &mut cfg)
}

pub fn main() -> ! {
    match run() {
        Ok(()) => {
            process::exit(0);
        },
        Err(ref err) => {
            if !err.is_silent() {
                println!("error: {}", err);
            }
            process::exit(err.exit_code());
        }
    }
}
