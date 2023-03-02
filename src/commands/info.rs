use std::io;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use serde::Serialize;

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::utils::logging::is_quiet_mode;
use crate::utils::system::QuietExit;

#[derive(Serialize, Default)]
pub struct AuthStatus {
    #[serde(rename = "type")]
    auth_type: Option<String>,
    successful: bool,
}

#[derive(Serialize, Default)]
pub struct ConfigStatus {
    org: Option<String>,
    project: Option<String>,
    url: Option<String>,
}

#[derive(Serialize, Default)]
pub struct Status {
    config: ConfigStatus,
    auth: AuthStatus,
    have_dsn: bool,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Print information about the configuration and verify authentication.")
        .arg(
            Arg::new("config_status_json")
                .long("config-status-json")
                .help(
                    "Return the status of the config that sentry-cli loads \
                     as JSON dump. This can be used by external tools to aid \
                     the user towards configuration.",
                ),
        )
        .arg(Arg::new("no_defaults").long("no-defaults").help(
            "Skip default organization and project checks. \
             This allows you to verify your authentication method, \
             without the need for setting other defaults.",
        ))
}

fn describe_auth(auth: Option<&Auth>) -> &str {
    match auth {
        None => "Unauthorized",
        Some(&Auth::Token(_)) => "Auth Token",
        Some(&Auth::Key(_)) => "API Key",
    }
}

fn get_config_status_json() -> Result<()> {
    let config = Config::current();
    let mut rv = Status::default();

    let (org, project) = config.get_org_and_project_defaults();
    rv.config.org = org;
    rv.config.project = project;
    rv.config.url = Some(config.get_base_url()?.to_string());

    rv.auth.auth_type = config.get_auth().map(|val| match val {
        Auth::Token(_) => "token".into(),
        Auth::Key(_) => "api_key".into(),
    });
    rv.auth.successful = config.get_auth().is_some() && Api::current().get_auth_info().is_ok();
    rv.have_dsn = config.get_dsn().is_ok();

    serde_json::to_writer_pretty(&mut io::stdout(), &rv)?;
    println!();
    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if matches.contains_id("config_status_json") {
        return get_config_status_json();
    }

    let config = Config::current();
    let (org, project) = config.get_org_and_project_defaults();
    let org = org.filter(|s| !s.is_empty());
    let project = project.filter(|s| !s.is_empty());
    let info_rv = Api::current().get_auth_info();
    let mut errors = config.get_auth().is_none() || info_rv.is_err();

    // If `no-defaults` is present, only authentication should be verified.
    if !matches.contains_id("no_defaults") {
        errors = errors || project.is_none() || org.is_none();
    }

    if is_quiet_mode() {
        return if errors {
            Err(QuietExit(1).into())
        } else {
            Ok(())
        };
    }

    println!("Sentry Server: {}", config.get_base_url().unwrap_or("-"));

    if !matches.contains_id("no_defaults") {
        println!(
            "Default Organization: {}",
            org.unwrap_or_else(|| "-".into())
        );
        println!("Default Project: {}", project.unwrap_or_else(|| "-".into()));
    }

    println!();
    println!("Authentication Info:");
    println!("  Method: {}", describe_auth(config.get_auth()));
    match info_rv {
        Ok(info) => {
            if let Some(ref user) = info.user {
                println!("  User: {}", user.email);
            }
            if let Some(ref auth) = info.auth {
                println!("  Scopes:");
                for scope in &auth.scopes {
                    println!("    - {scope}");
                }
            }
        }
        Err(err) => {
            println!("  (failure on authentication: {err})");
        }
    }

    if errors {
        Err(QuietExit(1).into())
    } else {
        Ok(())
    }
}
