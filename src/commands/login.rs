use std::env;

use anyhow::Result;
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
                // Convert to anyhow error to take advantage of anyhow's Debug impl
                let err = anyhow::anyhow!(err);

                if has_predefined_token {
                    return Err(err);
                } else {
                    println!("Error: {err:?}");
                }
            }
        }
    }

    let config_to_update = if matches.get_flag("global") {
        Config::global()?
    } else {
        Config::from_cli_config()?
    };

    if should_warn_about_overwrite(config_to_update.get_auth(), &token) {
        println!();
        println!("Warning: You are about to overwrite an existing token!");

        // Show organization information
        if let Some(existing_auth) = config_to_update.get_auth() {
            let existing_org = get_org_from_auth(existing_auth);
            let new_org = get_org_from_token(&token);

            println!("The current token is {}.", format_org_info(existing_org));
            println!("The new token is {}.", format_org_info(new_org));
        }

        println!();
        if !prompt_to_continue("Do you want to continue and overwrite the existing token?")? {
            println!("Token update cancelled.");
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

/// Helper function to get organization name from auth (treats keys and tokens without org as "(unknown)")
fn get_org_from_auth(auth: &Auth) -> Option<&str> {
    match auth {
        Auth::Token(token) => get_org_from_token(token),
        Auth::Key(_) => None,
    }
}

/// Helper function to get organization name from token
fn get_org_from_token(token: &AuthToken) -> Option<&str> {
    token.payload().map(|p| p.org.as_str())
}

/// Helper function to format organization information for display
fn format_org_info(org: Option<&str>) -> String {
    match org {
        Some(org_name) => format!("for organization {org_name}"),
        None => "not tied to any specific organization".to_string(),
    }
}

/// Helper function to determine if we should warn about overwriting an existing token
fn should_warn_about_overwrite(existing_auth: Option<&Auth>, new_token: &AuthToken) -> bool {
    // Only warn if there's an existing auth
    let Some(existing_auth) = existing_auth else {
        return false;
    };

    let existing_org = get_org_from_auth(existing_auth);
    let new_org = get_org_from_token(new_token);

    existing_org != new_org
}
