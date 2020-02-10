//! Implements a command for uninstalling `sentry-cli`
use std::env;
use std::fs;

use clap::{App, AppSettings, ArgMatches};
use console::style;
use failure::Error;

use crate::utils::system::{is_homebrew_install, is_npm_install, QuietExit};
use crate::utils::ui::prompt_to_continue;

fn is_hidden() -> bool {
    cfg!(windows) || is_homebrew_install() || is_npm_install()
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Uninstall the sentry-cli executable.")
        .settings(&if is_hidden() {
            vec![AppSettings::Hidden]
        } else {
            vec![]
        })
}

pub fn execute<'a>(_matches: &ArgMatches<'a>) -> Result<(), Error> {
    let exe = env::current_exe()?;

    if is_homebrew_install() {
        println!("This installation of sentry-cli is managed through homebrew");
        println!("Please use homebrew to uninstall sentry-cli");
        println!();
        println!("{} brew uninstall sentry-cli", style("$").dim());
        return Err(QuietExit(1).into());
    }
    if is_npm_install() {
        println!("This installation of sentry-cli is managed through npm/yarn");
        println!("Please use npm/yarn to uninstall sentry-cli");
        return Err(QuietExit(1).into());
    }
    if cfg!(windows) {
        println!("Cannot uninstall on Windows :(");
        println!();
        println!("Delete this file yourself: {}", exe.display());
        return Err(QuietExit(1).into());
    }

    if !prompt_to_continue("Do you really want to uninstall sentry-cli?")? {
        println!("Aborted!");
        return Ok(());
    }

    match fs::remove_file(&exe) {
        Ok(_) => println!("Uninstalled!"),
        Err(e) => {
            println!("Couldn't uninstall {}", exe.display());
            println!("Reason: {}", e);
            return Err(QuietExit(1).into());
        }
    }

    Ok(())
}
