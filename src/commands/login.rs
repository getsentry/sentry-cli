use std::env;

use anyhow::{bail, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use url::Url;

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::utils::auth_token::AuthToken;
use crate::utils::ui::{prompt, prompt_to_continue};

pub fn make_command(command: Command) -> Command {
    command.about("Authenticate with the Sentry server.").arg(
        Arg::new("global")
            .short('g')
            .long("global")
            .action(ArgAction::SetTrue)
            .help("Store authentication token globally rather than locally."),
    )
}

fn update_config(config: &Config, token: AuthToken) -> Result<()> {
    let mut new_cfg = config.clone();
    new_cfg.set_auth(Auth::Token(token));
    new_cfg.save()?;
    Ok(())
}

fn get_org_from_auth(auth: &Auth) -> &str {
    match auth {
        Auth::Token(token) => token.payload()
            .map(|p| p.org.as_str())
            .unwrap_or("(unknown)"),
        Auth::Key(_) => "(unknown)",
    }
}

fn should_warn_about_overwrite(existing_auth: Option<&Auth>) -> bool {
    // Warn if there's any existing auth (token or key)
    existing_auth.is_some()
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let token_url = format!(
        "{}/orgredirect/organizations/:orgslug/settings/auth-tokens/",
        config.get_base_url()?
    );
    let predefined_token = matches.get_one::<AuthToken>("auth_token");
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
            token.to_owned()
        } else {
            prompt("Enter your token")?.into()
        };

        let test_cfg = config.make_copy(|cfg| {
            cfg.set_auth(Auth::Token(token.clone()));
            Ok(())
        })?;

        match Api::with_config(test_cfg).authenticated()?.get_auth_info() {
            Ok(info) => {
                match info.user {
                    Some(user) => {
                        // Old school user auth token
                        println!("Valid token for user {}", user.email);
                    }
                    None => {
                        // New org auth token
                        println!("Valid org token");
                    }
                }
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

    let config_to_update = if matches.get_flag("global") {
        Config::global()?
    } else {
        Config::from_cli_config()?
    };

    if should_warn_about_overwrite(config_to_update.get_auth()) {
        let existing_org = config_to_update.get_auth()
            .map(get_org_from_auth)
            .unwrap_or("(none)");
        let new_org = token.payload()
            .map(|p| p.org.as_str())
            .unwrap_or("(unknown)");
        
        println!();
        println!("Warning: Overwriting existing token");
        println!("  Current org: {}", existing_org);
        println!("  New org: {}", new_org);
        
        if !prompt_to_continue("Continue?")? {
            return Ok(());
        }
    }

    update_config(&config_to_update, token)?;
    println!();
    println!(
        "Stored token in {}",
        config_to_update.get_filename().display()
    );

    Ok(())
}
