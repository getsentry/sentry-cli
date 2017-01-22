//! Implements a command for uninstalling `sentry-cli`
use std::env;

use clap::{App, ArgMatches};

use prelude::*;
use config::Config;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uninstalls the sentry-cli executable")
}

#[cfg(windows)]
pub fn execute<'a>(_matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    let exe = env::current_exe()?;
    println!("Cannot uninstall on Windows :(");
    println!("");
    println!("Delete this file yourself: {}", exe.display());
    Ok(())
}

#[cfg(not(windows))]
pub fn execute<'a>(_matches: &ArgMatches<'a>, _config: &Config) -> Result<()> {
    use std::fs;
    use runas;
    use utils;

    let exe = env::current_exe()?;

    if !utils::prompt_to_continue("Do you really want to uninstall sentry-cli?")? {
        println!("Aborted!");
        return Ok(());
    }

    if !utils::is_writable(&exe) {
        println!("Need to sudo to uninstall {}", exe.display());
        runas::Command::new("rm").arg("-f")
            .arg(&exe)
            .status()?;
    } else {
        fs::remove_file(&exe)?;
    }
    println!("Uninstalled!");

    Ok(())
}
