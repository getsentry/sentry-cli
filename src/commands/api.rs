use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command, ValueHint};

use crate::api::{Api, Method};

pub fn make_command(command: Command) -> Command {
    command
        .about("Make a raw API request to the Sentry API.")
        .arg(
            Arg::new("endpoint")
                .value_name("ENDPOINT")
                .required(true)
                .value_hint(ValueHint::Url)
                .help(
                    "The API endpoint to request (e.g., 'organizations/' or '/projects/my-org/my-project/releases/').{n}\
                     The endpoint will be prefixed with '/api/0/' automatically.",
                ),
        )
        .arg(
            Arg::new("method")
                .short('m')
                .long("method")
                .value_name("METHOD")
                .value_parser(["GET", "POST", "PUT", "DELETE", "PATCH"])
                .default_value("GET")
                .action(ArgAction::Set)
                .help("The HTTP method to use for the request."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    let method_str = matches.get_one::<String>("method").unwrap();

    let method = match method_str.as_str() {
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "DELETE" => Method::Delete,
        "PATCH" => Method::Patch,
        _ => unreachable!("Invalid method value"),
    };

    let api = Api::current();
    let resp = api.request(method, endpoint, None)?.send()?;

    // Print the response body as-is to stdout
    if let Some(body) = resp.body {
        let body_str = String::from_utf8_lossy(&body);
        println!("{}", body_str);
    }

    Ok(())
}
