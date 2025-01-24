use std::env;

use anyhow::{bail, Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::utils::update::{assert_updatable, can_update_sentrycli, get_latest_sentrycli_release};

pub fn make_command(command: Command) -> Command {
    let command = command.about("Update the sentry-cli executable.").arg(
        Arg::new("force")
            .long("force")
            .short('f')
            .action(ArgAction::SetTrue)
            .help("Force the update even if the latest version is already installed."),
    );

    if can_update_sentrycli() {
        command.hide(true)
    } else {
        command
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    // Disable update check in case of errors
    env::set_var("SENTRY_DISABLE_UPDATE_CHECK", "true");

    // Aborts with an error if this installation is not updatable.
    assert_updatable()?;

    let exe = env::current_exe()?;
    let update = get_latest_sentrycli_release()
        .with_context(|| "Error getting latest Sentry CLI version.")?;
    if !update.have_version_info() {
        bail!("Could not get the latest release version.");
    }

    println!("Latest release is {}", update.latest_version());

    // It's not currently possible to easily mock I/O with `trycmd`,
    // but verifying that `execute` is not panicking, is good enough for now.
    if env::var("SENTRY_INTEGRATION_TEST").is_ok() {
        println!("Running in integration tests mode. Skipping execution.");
        return Ok(());
    }

    if update.is_latest_version() {
        if matches.get_flag("force") {
            println!("Forcing update");
        } else {
            println!("Already up to date!");
            return Ok(());
        }
    }

    println!("Updating executable at {}", exe.display());
    update.download()?;
    println!("Updated to {}!", update.latest_version());
    Ok(())
}
