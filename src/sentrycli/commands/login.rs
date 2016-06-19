use clap::{App, ArgMatches};
use open;
use url::Url;
use std::fs::OpenOptions;

use CliResult;
use commands::{Config, Auth};
use utils::{prompt, prompt_to_continue};
use api::Api;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("helper that assists in signin in with sentry.")
}

fn update_config(config: &Config, token: &str) -> CliResult<()> {
    let mut new_cfg = config.clone();

    new_cfg.ini.set_to(Some("defaults"), "url".into(), config.url.clone());
    new_cfg.ini.set_to(Some("auth"), "token".into(), token.into());
    new_cfg.ini.delete_from(Some("auth"), "api_key");

    let mut file = OpenOptions::new()
        .write(true).truncate(true).create(true).open(&new_cfg.filename)?;
    new_cfg.ini.write_to(&mut file)?;

    Ok(())
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    let token_url = format!("{}/api/", config.url.trim_right_matches('/'));

    println!("This helps you signing in your sentry-cli with an authentication token.");
    println!("If you do not yet have a token ready we can bring up a browser for you");
    println!("to create a token now.");
    println!("");
    println!("Sentry server: {}", Url::parse(&config.url)?
             .host_str().unwrap_or("<unknown>"));

    if prompt_to_continue("Open browser now?")? {
        if open::that(&token_url).is_err() {
            println!("Cannot open browser. Please manually go to {}", &token_url);
        }
    }

    let mut token;
    loop {
        token = prompt("Enter your token")?;

        let mut test_cfg = config.clone();
        test_cfg.auth = Auth::Token(token.clone());
        match Api::new(&test_cfg).get_auth_info() {
            Ok(info) => {
                // we can unwrap here somewhat safely because we do not permit
                // signing in with legacy non user bound api keys here.
                println!("Valid token for user {}", info.user.unwrap().email);
                break;
            },
            Err(err) => {
                println!("Invalid token: {}", err);
            }
        }
    }

    update_config(&config, &token)?;
    println!("");
    println!("Stored token in {}", config.filename.display());

    Ok(())
}
