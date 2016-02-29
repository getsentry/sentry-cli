use std::env;
use std::process;
use std::io;

use clap::{Arg, App, AppSettings};
use hyper::client::request::Request;
use hyper::client::response::Response;
use hyper::header::{Authorization, Basic, ContentType, ContentLength};
use hyper::method::Method;
use hyper::net::Fresh;
use url::Url;
use serde::Serialize;
use serde_json;

use CliResult;
use utils::make_subcommand;

#[derive(Debug)]
pub struct Config {
    api_key: Option<String>,
    url: String,
}

impl Config {

    pub fn prepare_api_request(&self, method: Method, path: &str)
        -> CliResult<Request<Fresh>>
    {
        let url = try!(Url::parse(&format!(
            "{}/api/0{}", self.url.trim_right_matches("/"), path)));
        let mut req = try!(Request::new(method, url));
        {
            if let Some(ref api_key) = self.api_key {
                req.headers_mut().set(Authorization(Basic {
                    username: api_key.clone(),
                    password: None
                }));
            }
        }
        Ok(req)
    }

    pub fn api_request(&self, method: Method, path: &str)
        -> CliResult<Response>
    {
        let req = try!(self.prepare_api_request(method, path));
        Ok(try!(try!(req.start()).send()))
    }

    pub fn json_api_request<T: Serialize>(&self, method: Method, path: &str, body: &T)
        -> CliResult<Response>
    {
        let mut req = try!(self.prepare_api_request(method, path));
        let mut body_bytes : Vec<u8> = vec![];
        try!(serde_json::to_writer(&mut body_bytes, &body));

        {
            let mut headers = req.headers_mut();
            headers.set(ContentType(mime!(Application/Json)));
            headers.set(ContentLength(body_bytes.len() as u64));
        }

        let mut req = try!(req.start());
        try!(io::copy(&mut &body_bytes[..], &mut req));
        Ok(try!(req.send()))
    }
}

macro_rules! each_subcommand {
    ($mac:ident) => {
        $mac!(upload_dsym);
        $mac!(extract_iosds_symbols);
        $mac!(releases);
    }
}

macro_rules! import_subcommand {
    ($name:ident) => { mod $name; }
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
        .arg(Arg::with_name("api_key")
             .value_name("API_KEY")
             .long("api-key")
             .help("The sentry API key to use"));

    macro_rules! add_subcommand {
        ($name:ident) => {{
            app = app.subcommand($name::make_app(
                make_subcommand(&stringify!($name).replace("_", "-"))));
        }}
    }
    each_subcommand!(add_subcommand);

    let matches = try!(app.get_matches_from_safe(args));

    if let Some(url) = matches.value_of("url") {
        config.url = url.to_owned();
    }
    if let Some(api_key) = matches.value_of("api_key") {
        config.api_key = Some(api_key.to_owned());
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
    execute(env::args().collect(), &mut Config {
        api_key: env::var("SENTRY_TOKEN").ok(),
        url: "https://api.getsentry.com/".to_owned(),
    })
}

pub fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(ref err) => err.exit(),
    }
}
