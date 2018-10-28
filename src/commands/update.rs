//! Implements a command for updating `sentry-cli`
use std::env;

use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Error;

use crate::utils::update::{can_update_sentrycli, get_latest_sentrycli_release};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Update the sentry-cli executable.")
        .settings(&if !can_update_sentrycli() {
            vec![AppSettings::Hidden]
        } else {
            vec![]
        }).arg(
            Arg::with_name("force")
                .long("force")
                .short("f")
                .help("Force the update even if the latest version is already installed."),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let exe = env::current_exe()?;
    let update = get_latest_sentrycli_release()?;

    // aborts with an error if this installation is not updatable.
    update.assert_updatable()?;

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
