use std::env;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use url::Url;

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::utils::ui::{prompt, prompt_to_continue};

pub fn make_command(command: Command) -> Command {
    command.about("Authenticate with the Sentry server.").arg(
        Arg::new("global")
            .short('g')
            .long("global")
            .help("Store authentication token globally rather than locally."),
    )
}

fn update_config(config: &Config, token: &str) -> Result<()> {
    let mut new_cfg = config.clone();
    new_cfg.set_auth(Auth::Token(token.to_string()));
    new_cfg.save()?;
    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let token_url = format!("{}/api/", config.get_base_url()?);
    let predefined_token = matches.get_one::<String>("auth_token");
    let has_predefined_token = predefined_token.is_some();

    println!("This helps you signing in your sentry-cli with an authentication token.");
    println!("If you do not yet have a token ready we can bring up a browser for you");
    println!("to create a token now.");
    println!();
    println!(
        "Sentry server: {}",
        Url::parse(config.get_base_url()?)?
            .host_str()
            .unwrap_or("<unknown>")
    );

    // It's not currently possible to easily mock I/O with `trycmd`,
    // but verifying that `execute` is not panicking, is good enough for now.
    if env::var("SENTRY_INTEGRATION_TEST").is_ok() {
        println!("Running in integration tests mode. Skipping execution.");
        return Ok(());
    }

    if !has_predefined_token
        && prompt_to_continue("Open browser now?")?
        && open::that(&token_url).is_err()
    {
        println!("Cannot open browser. Please manually go to {}", &token_url);
    }

    let mut token;
    loop {
        token = if let Some(token) = predefined_token {
            token.to_string()
        } else {
            prompt("Enter your token")?
        };

        let test_cfg = config.make_copy(|cfg| {
            cfg.set_auth(Auth::Token(token.to_string()));
            Ok(())
        })?;
        match Api::with_config(test_cfg).get_auth_info() {
            Ok(info) => {
                // we can unwrap here somewhat safely because we do not permit
                // signing in with legacy non user bound api keys here.
                println!("Valid token for user {}", info.user.unwrap().email);
                break;
            }
            Err(err) => {
                let msg = format!("Invalid token: {err}");
                if has_predefined_token {
                    bail!(msg);
                } else {
                    println!("{msg}");
                }
            }
        }
    }

    let config_to_update = if matches.contains_id("global") {
        Config::global()?
    } else {
        Config::from_cli_config()?
    };

    update_config(&config_to_update, &token)?;
    println!();
    println!(
        "Stored token in {}",
        config_to_update.get_filename().display()
    );

    Ok(())
}
