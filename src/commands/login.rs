//! Implements a command for signing in.
use clap::{App, Arg, ArgMatches};
use failure::Error;
use url::Url;

use crate::api::Api;
use crate::config::{Auth, Config};
use crate::utils::ui::{prompt, prompt_to_continue};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Authenticate with the Sentry server.").arg(
        Arg::with_name("global")
            .short("g")
            .long("global")
            .help("Store authentication token globally rather than locally."),
    )
}

fn update_config(config: &Config, token: &str) -> Result<(), Error> {
    let mut new_cfg = config.clone();
    new_cfg.set_auth(Auth::Token(token.to_string()))?;
    new_cfg.save()?;
    Ok(())
}

pub fn execute(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let config = Config::current();
    let token_url = format!(
        "{}/orgredirect/organizations/:orgslug/settings/auth-tokens/",
        config.get_base_url()?
    );

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

    if prompt_to_continue("Open browser now?")? && open::that(&token_url).is_err() {
        println!("Cannot open browser. Please manually go to {}", &token_url);
    }

    let mut token;
    loop {
        token = prompt("Enter your token")?;

        let test_cfg = config.make_copy(|cfg| {
            cfg.set_auth(Auth::Token(token.to_string()))?;
            Ok(())
        })?;

        match Api::with_config(test_cfg).get_auth_info() {
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
                println!("Invalid token: {err}");
            }
        }
    }

    let config_to_update = if matches.is_present("global") {
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
