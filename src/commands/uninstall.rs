use std::env;
use std::fs;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use console::style;

use crate::utils::fs::is_writable;
use crate::utils::system::{is_homebrew_install, is_npm_install, QuietExit};
use crate::utils::ui::prompt_to_continue;

pub fn make_command(command: Command) -> Command {
    let command = command.about("Uninstall the sentry-cli executable.").arg(
        Arg::new("confirm")
            .long("confirm")
            .help("Skip uninstall confirmation prompt."),
    );

    if cfg!(windows) || is_homebrew_install() || is_npm_install() {
        command.hide(true)
    } else {
        command
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
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
        println!(
            "Please use npm/yarn to uninstall sentry-cli, using one of the following commands:"
        );
        println!("  yarn remove @sentry/cli");
        println!("  yarn global remove @sentry/cli");
        println!("  npm uninstall @sentry/cli");
        println!("  npm uninstall --global @sentry/cli");
        return Err(QuietExit(1).into());
    }
    if cfg!(windows) {
        println!("Cannot uninstall on Windows :(");
        println!();
        println!("Delete this file yourself: {}", exe.display());
        return Err(QuietExit(1).into());
    }

    // It's not currently possible to easily mock I/O with `trycmd`,
    // but verifying that `execute` is not panicking, is good enough for now.
    if env::var("SENTRY_INTEGRATION_TEST").is_ok() {
        println!("Running in integration tests mode. Skipping execution.");
        return Ok(());
    }

    if !matches.contains_id("confirm")
        && !prompt_to_continue("Do you really want to uninstall sentry-cli?")?
    {
        println!("Aborted!");
        return Ok(());
    }

    if !is_writable(&exe) {
        println!("Need to sudo to uninstall {}", exe.display());
        runas::Command::new("rm").arg("-f").arg(exe).status()?;
    } else {
        fs::remove_file(&exe)?;
    }
    println!("Uninstalled!");

    Ok(())
}
