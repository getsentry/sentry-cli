//! Implements a command for showing infos from Sentry.
use crates::clap::{App, ArgMatches};

use prelude::*;
use api::Api;
use config::{Auth, Config};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("print out information about the sentry server")
}

fn describe_auth(auth: Option<&Auth>) -> &str {
    match auth {
        None => "Unauthorized",
        Some(&Auth::Token(_)) => "Auth Token",
        Some(&Auth::Key(_)) => "API Key",
    }
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let (project, org) = config.get_org_and_project_defaults();
    let info_rv = Api::new(config).get_auth_info();

    println!("Sentry Server: {}", config.url);
    println!("Default Organization: {}", project.unwrap_or("-".into()));
    println!("Default Project: {}", org.unwrap_or("-".into()));

    if config.auth.is_some() {
        println!("");
        println!("Authentication Info:");
        println!("  Method: {}", describe_auth(config.auth.as_ref()));
        match info_rv {
            Ok(info) => {
                if let Some(ref user) = info.user {
                    println!("  User: {}", user.email);
                }
                if let Some(ref auth) = info.auth {
                    println!("  Scopes:");
                    for scope in &auth.scopes {
                        println!("    - {}", scope);
                    }
                }
            }
            Err(err) => {
                println!("  (failure on authentication: {})", err);
            }
        }
    } else {
    }

    Ok(())
}
