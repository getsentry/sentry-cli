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
pub enum Auth {
    Token(String),
    SystemAdminPassword(String)
}

#[derive(Debug)]
pub struct Config {
    auth: Option<Auth>,
    url: String,
}

impl Config {

    pub fn api_request(&self, method: Method, path: &str)
            -> CliResult<Request<Fresh>> {
        let url = try!(Url::parse(&format!(
            "{}/api/0{}", self.url.trim_right_matches("/"), path)));
        let mut request = try!(Request::new(method, url));
        {
            let mut headers = request.headers_mut();
            match self.auth {
                None => fail!("Missing authentication"),
                Some(ref auth) => {
                    match *auth {
                        Auth::Token(ref token) => {
                            headers.set(Authorization(Basic {
                                username: token.clone(),
                                password: None
                            }));
                        },
                        Auth::SystemAdminPassword(ref pw) => {
                            headers.set(Authorization(Basic {
                                username: "admin".to_owned(),
                                password: Some(pw.clone())
                            }));
                        }
                    }
                }
            }
        }
        Ok(request)
    }
}

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dsym);
        $mac!(extract_iosds_symbols);
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
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(Arg::with_name("url")
             .value_name("URL")
             .long("url")
             .help("The sentry API URL"))
        .arg(Arg::with_name("system_admin_password")
             .value_name("PASSWORD")
             .long("system-admin-password")
             .help("Sign in as system super administrator."))
        .arg(Arg::with_name("token")
             .value_name("TOKEN")
             .long("token")
             .help("The sentry API token to use"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand($name::make_app(
                App::new(stringify!($name).replace("_", "-"))
                    .setting(AppSettings::UnifiedHelpMessage)));
        }}
    }

    each_subcommand!(add_subcommand);

    let matches = try!(app.get_matches_from_safe(args));

    if let Some(url) = matches.value_of("url") {
        config.url = url.to_owned();
    }
    if let Some(token) = matches.value_of("token") {
        config.auth = Some(Auth::Token(token.to_owned()));
    }
    if let Some(password) = matches.value_of("system_admin_password") {
        config.auth = Some(Auth::SystemAdminPassword(password.to_owned()));
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
    let auth = if let Ok(token) = env::var("SENTRY_TOKEN") {
        Some(Auth::Token(token.to_owned()))
    } else if let Ok(password) = env::var("SENTRY_SYSTEM_ADMIN_PASSWORD") {
        Some(Auth::SystemAdminPassword(password.to_owned()))
    } else {
        None
    };
    let mut cfg = Config {
        auth: auth,
        url: "https://api.getsentry.com/".to_owned(),
    };
    execute(env::args().collect(), &mut cfg)
}

pub fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(ref err) => err.exit(),
    }
}
