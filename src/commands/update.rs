//! Implements a command for updating `sentry-cli`
use std::env;

use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Error;

use crate::utils::update::{assert_updatable, can_update_sentrycli, get_latest_sentrycli_release};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Update the sentry-cli executable.")
        .settings(&if !can_update_sentrycli() {
            vec![AppSettings::Hidden]
        } else {
            vec![]
        })
        .arg(
            Arg::with_name("force")
                .long("force")
                .short("f")
                .help("Force the update even if the latest version is already installed."),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    // Disable update check in case of errors
    env::set_var("SENTRY_DISABLE_UPDATE_CHECK", "true");

    // Aborts with an error if this installation is not updatable.
    assert_updatable()?;

    let exe = env::current_exe()?;
    let update = get_latest_sentrycli_release()?;
    if !update.have_version_info() {
        bail!("Could not get the latest release version.");
    }

    println!("Latest release is {}", update.latest_version());
    if update.is_latest_version() {
        if matches.is_present("force") {
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
