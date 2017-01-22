//! Implements a command for updating `sentry-cli`
use std::fs;
use std::env;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use runas;

use prelude::*;
use api::Api;
use utils;
use config::Config;
use constants::VERSION;


#[cfg(windows)]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> Result<()> {
    // so on windows you can rename a running executable but you cannot delete it.
    // we move the old executable to a temporary location (this most likely only
    // works if they are on the same FS) and then put the new in place.  This
    // will leave the old executable in the temp path lying around so let's hope
    // that windows cleans up temp files there (spoiler: it does not)
    let tmp = env::temp_dir().join(".sentry-cli.tmp");

    if elevate {
        runas::Command::new("cmd").arg("/c")
            .arg("move")
            .arg(&exe)
            .arg(&tmp)
            .arg("&")
            .arg("move")
            .arg(&downloaded_path)
            .arg(&exe)
            .arg("&")
            .arg("del")
            .arg(&tmp)
            .status()?;
    } else {
        fs::rename(&exe, &tmp)?;
        fs::rename(&downloaded_path, &exe)?;
        fs::remove_file(&tmp).ok();
    }

    Ok(())
}

#[cfg(not(windows))]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> Result<()> {
    if elevate {
        println!("Need to sudo to overwrite {}", exe.display());
        runas::Command::new("mv").arg(&downloaded_path)
            .arg(&exe)
            .status()?;
    } else {
        fs::rename(&downloaded_path, &exe)?;
    }
    Ok(())
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("update the sentry-cli executable")
        .arg(Arg::with_name("force")
            .long("force")
            .short("f")
            .help("Force the update even if already current."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let api = Api::new(config);
    let exe = env::current_exe()?;
    let elevate = !utils::is_writable(&exe);

    info!("expecting elevation for update: {}", elevate);

    let latest_release = match api.get_latest_sentrycli_release()? {
        Some(release) => release,
        None => fail!("Could not find download URL for updates."),
    };
    let tmp_path = if elevate {
        env::temp_dir().join(".sentry-cli.part")
    } else {
        exe.parent().unwrap().join(".sentry-cli.part")
    };

    println!("Latest release is {}", latest_release.version);
    if latest_release.version == VERSION {
        if matches.is_present("force") {
            println!("Forcing update");
        } else {
            println!("Already up to date!");
            return Ok(());
        }
    }

    println!("Updating executable at {}", exe.display());

    let mut f = fs::File::create(&tmp_path)?;
    match api.download(&latest_release.download_url, &mut f) {
        Ok(_) => {}
        Err(err) => {
            fs::remove_file(tmp_path).ok();
            fail!(err);
        }
    };

    utils::set_executable_mode(&tmp_path)?;

    rename_exe(&exe, &tmp_path, elevate)?;

    println!("Updated!");

    Ok(())
}
