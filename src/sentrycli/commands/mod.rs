use std::env;
use std::process;

use clap::{Arg, App, AppSettings};
use hyper::client::request::Request;
use hyper::header::{Authorization, Basic};
use hyper::method::Method;
use hyper::net::Fresh;
use url::Url;

use super::CliResult;

#[derive(Debug)]
pub struct Config {
    token: String,
    url: String,
}

impl Config {

    pub fn api_request(&self, method: Method, path: &str) -> CliResult<Request<Fresh>> {
        let url = try!(Url::parse(&format!("{}/api/0{}", self.url.trim_right_matches("/"), path)));
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
    ($mac:ident) => {
        $mac!(upload_dsym);
    }
}

macro_rules! import_subcommand {
    ($name:ident) => {
        mod $name;
    }
}

each_subcommand!(import_subcommand);

pub fn execute(args: Vec<String>, config: &mut Config) -> CliResult<()> {
    let mut app = App::new("sentry-cli")
        .author("Sentry")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Command line utility for Sentry")
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("url")
             .value_name("URL")
             .long("url")
             .help("The sentry API URL"))
        .arg(Arg::with_name("token")
             .value_name("TOKEN")
             .long("token")
             .help("The sentry API token to use"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            let mut sub_app = App::new(cmd).setting(AppSettings::UnifiedHelpMessage);
            sub_app = $name::make_app(sub_app);
            app = app.subcommand(sub_app);
        }}
    }

    each_subcommand!(add_subcommand);

    let matches = try!(app.get_matches_from_safe(args));

    if let Some(url) = matches.value_of("url") {
        config.url = url.to_owned();
    }
    if let Some(token) = matches.value_of("token") {
        config.token = token.to_owned();
    }

    macro_rules! execute_subcommand {
        ($name:ident) => {{
            let cmd = stringify!($name).replace("_", "-");
            if let Some(sub_matches) = matches.subcommand_matches(cmd) {
                return $name::execute(&sub_matches, &config);
            }
        }}
    }
    each_subcommand!(execute_subcommand);

    panic!("Should never reach this point");
}

pub fn run() -> CliResult<()> {
    let mut cfg = Config {
        token: env::var("SENTRY_TOKEN").unwrap_or("".to_owned()),
        url: "https://api.getsentry.com/".to_owned(),
    };
    execute(env::args().collect(), &mut cfg)
}

pub fn main() -> ! {
    match run() {
        Ok(()) => {
            process::exit(0);
        },
        Err(ref err) => {
            err.exit();
        }
    }
}
