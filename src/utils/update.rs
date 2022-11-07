use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

use anyhow::{bail, format_err, Result};
use chrono::{DateTime, Duration, Utc};
use console::{style, user_attended};
use if_chain::if_chain;
use log::{debug, info};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::api::{Api, SentryCliRelease};
use crate::config::Config;
use crate::constants::{APP_NAME, VERSION};
use crate::utils::fs::{is_writable, set_executable_mode};
use crate::utils::system::{is_homebrew_install, is_npm_install, QuietExit};

#[cfg(windows)]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> Result<()> {
    // so on windows you can rename a running executable but you cannot delete it.
    // we move the old executable to a temporary location (this most likely only
    // works if they are on the same FS) and then put the new in place.  This
    // will leave the old executable in the temp path lying around so let's hope
    // that windows cleans up temp files there (spoiler: it does not)
    let tmp = env::temp_dir().join(".sentry-cli.tmp");

    if elevate {
        runas::Command::new("cmd")
            .arg("/c")
            .arg("move")
            .arg(exe)
            .arg(&tmp)
            .arg("&")
            .arg("move")
            .arg(downloaded_path)
            .arg(exe)
            .arg("&")
            .arg("del")
            .arg(tmp)
            .status()?;
    } else {
        fs::rename(exe, &tmp)?;
        fs::rename(downloaded_path, exe)?;
        fs::remove_file(&tmp).ok();
    }

    Ok(())
}

#[cfg(not(windows))]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> Result<()> {
    if elevate {
        println!("Need to sudo to overwrite {}", exe.display());
        runas::Command::new("mv")
            .arg(downloaded_path)
            .arg(exe)
            .status()?;
    } else {
        fs::rename(downloaded_path, exe)?;
    }
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
pub struct LastUpdateCheck {
    pub last_check_timestamp: Option<DateTime<Utc>>,
    pub last_check_version: Option<String>,
    pub last_fetched_version: Option<String>,
}

impl LastUpdateCheck {
    pub fn update_for_info(&mut self, ui: &SentryCliUpdateInfo) {
        self.last_check_timestamp = Some(Utc::now());
        self.last_check_version = Some(ui.current_version().to_string());
        self.last_fetched_version = Some(ui.latest_version().to_string());
    }

    pub fn should_run_check(&self) -> bool {
        if_chain! {
            if let Some(ts) = self.last_check_timestamp;
            if let Some(ref ver) = self.last_check_version;
            then {
                ver.as_str() != VERSION || ts < Utc::now() - Duration::hours(12)
            } else {
                true
            }
        }
    }

    pub fn is_outdated(&self) -> bool {
        if_chain! {
            if let Some(ref release_v) = self.last_fetched_version;
            if let Some(ref check_v) = self.last_check_version;
            then {
                Version::parse(release_v.as_str()).unwrap() < Version::parse(VERSION).unwrap() &&
                check_v.as_str() == VERSION
            } else {
                false
            }
        }
    }

    pub fn latest_version(&self) -> &str {
        self.last_fetched_version.as_deref().unwrap_or("0.0")
    }
}

pub struct SentryCliUpdateInfo {
    latest_release: Option<SentryCliRelease>,
}

impl SentryCliUpdateInfo {
    pub fn have_version_info(&self) -> bool {
        self.latest_release.is_some()
    }

    pub fn is_latest_version(&self) -> bool {
        self.latest_version() == VERSION
    }

    pub fn current_version(&self) -> &str {
        VERSION
    }

    pub fn latest_version(&self) -> &str {
        if let Some(ref rel) = self.latest_release {
            rel.version.as_str()
        } else {
            "0.0"
        }
    }

    pub fn download_url(&self) -> Result<&str> {
        if let Some(ref rel) = self.latest_release {
            Ok(rel.download_url.as_str())
        } else {
            bail!("Could not get download URL for latest release.");
        }
    }

    pub fn download(&self) -> Result<()> {
        let exe = env::current_exe()?;
        let elevate = !is_writable(&exe);
        info!("expecting elevation for update: {}", elevate);
        let tmp_path = if elevate {
            env::temp_dir().join(".sentry-cli.part")
        } else {
            exe.parent().unwrap().join(".sentry-cli.part")
        };
        let mut f = fs::File::create(&tmp_path)?;
        let api = Api::current();
        match api.download_with_progress(self.download_url()?, &mut f) {
            Ok(_) => {}
            Err(err) => {
                fs::remove_file(tmp_path).ok();
                bail!(err);
            }
        };

        set_executable_mode(&tmp_path)?;
        rename_exe(&exe, &tmp_path, elevate)?;
        Ok(())
    }
}

pub fn get_latest_sentrycli_release() -> Result<SentryCliUpdateInfo> {
    let api = Api::current();
    Ok(SentryCliUpdateInfo {
        latest_release: if let Ok(release) = api.get_latest_sentrycli_release() {
            release
        } else {
            None
        },
    })
}

pub fn can_update_sentrycli() -> bool {
    !is_homebrew_install() && !is_npm_install()
}

pub fn assert_updatable() -> Result<()> {
    if is_homebrew_install() {
        println!("This installation of sentry-cli is managed through homebrew");
        println!("Please use homebrew to update sentry-cli:");
        println!();
        println!("{} brew upgrade sentry-cli", style("$").dim());
        return Err(QuietExit(1).into());
    } else if is_npm_install() {
        println!("This installation of sentry-cli is managed through npm/yarn");
        println!("Please use npm/yarn to update sentry-cli, using one of the following commands:");
        println!("  yarn upgrade @sentry/cli");
        println!("  yarn global upgrade @sentry/cli");
        println!("  npm update @sentry/cli");
        println!("  npm update -g @sentry/cli");
        return Err(QuietExit(1).into());
    }
    Ok(())
}

fn update_nagger_impl() -> Result<()> {
    let mut path = dirs::cache_dir().ok_or_else(|| format_err!("Could not get cache folder"))?;

    path.push(APP_NAME);
    fs::create_dir_all(path.clone())?;
    path.push("updatecheck");

    let mut check: LastUpdateCheck = if let Ok(f) = fs::File::open(&path) {
        serde_json::from_reader(io::BufReader::new(f))?
    } else {
        Default::default()
    };

    if check.should_run_check() {
        info!("Running update nagger update check");
        let ui = get_latest_sentrycli_release()?;
        if ui.have_version_info() {
            check.update_for_info(&ui);
            let mut f = fs::File::create(&path)?;
            serde_json::to_writer_pretty(&mut f, &check)?;
            f.write_all(b"\n")?;
        }
    } else {
        info!("Skipping update nagger update check");
    }

    if check.is_outdated() {
        info!("Update nagger determined outdated installation");
        eprintln!();
        eprintln!(
            "{}",
            style(format!(
                "sentry-cli update to {} is available!",
                check.latest_version()
            ))
            .yellow()
        );
        if is_homebrew_install() {
            eprintln!("{}", style("run brew upgrade sentry-cli to update").dim());
        } else if is_npm_install() {
            eprintln!(
                "{}",
                style("Please use npm/yarn to update sentry-cli").dim()
            )
        } else {
            eprintln!("{}", style("run sentry-cli update to update").dim());
        }
    }

    Ok(())
}

pub fn run_sentrycli_update_nagger() {
    let config = match Config::current_opt() {
        Some(config) => config,
        None => return,
    };

    // Only update if we are compiled as unmanaged version (default)
    if cfg!(feature = "managed") {
        return;
    }

    // Do not run update nagger if stdout/stdin is not a terminal
    if !user_attended() {
        debug!("skipping update nagger because session is not attended");
        return;
    }

    // npm installs do not get an update check.  We might want to relax this later
    // to support update checks for global npm installs but not dependency installs.
    if is_npm_install() {
        return;
    }

    // if the update nagger is disabled, do not run it.
    if config.disable_update_nagger() {
        info!("update nagger was disabled, not running update checks");
        return;
    }

    update_nagger_impl().ok();
}
