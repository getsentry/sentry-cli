//! Implements a command for uninstalling `sentry-cli`
use std::env;
use std::fs;

use clap::{App, AppSettings, Arg, ArgMatches};
use console::style;
use failure::Error;

use crate::utils::fs::is_writable;
use crate::utils::system::{is_homebrew_install, is_npm_install, QuietExit};
use crate::utils::ui::prompt_to_continue;

fn is_hidden() -> bool {
    cfg!(windows) || is_homebrew_install() || is_npm_install()
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Uninstall the sentry-cli executable.")
        .arg(
            Arg::with_name("confirm")
                .long("confirm")
                .help("Skip uninstall confirmation prompt."),
        )
        .settings(&if is_hidden() {
            vec![AppSettings::Hidden]
        } else {
            vec![]
        })
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
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

    if !matches.is_present("confirm") {
        if !prompt_to_continue("Do you really want to uninstall sentry-cli?")? {
            println!("Aborted!");
            return Ok(());
        }
    }

    if !is_writable(&exe) {
        println!("Need to sudo to uninstall {}", exe.display());
        runas::Command::new("rm").arg("-f").arg(&exe).status()?;
    } else {
        fs::remove_file(&exe)?;
    }
    println!("Uninstalled!");

    Ok(())
}
