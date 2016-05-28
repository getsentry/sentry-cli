use std::fs;
use std::env;
use std::process::Command;

use clap::{App, ArgMatches};

use utils;
use CliResult;
use commands::Config;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("uninstalls the sentry-cli executable")
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, _config: &Config) -> CliResult<()> {
    let exe = env::current_exe()?;

    if !utils::prompt_to_continue("Do you really want to uninstall sentry-cli?")? {
        println!("Aborted!");
        return Ok(());
    }

    if !utils::is_writable(&exe) {
        println!("Need to sudo to uninstall {}", exe.display());
        Command::new("sudo")
            .arg("-k")
            .arg("rm -f")
            .arg(&exe)
            .status()?;
    } else {
        fs::remove_file(&exe)?;
    }
    println!("Uninstalled!");

    Ok(())
}
