use std::fs;
use std::env;
use std::path::Path;
use runas;
use console::style;

use config::Config;
use api::{Api, SentryCliRelease};
use constants::VERSION;
use utils::{is_homebrew_install, is_npm_install, set_executable_mode, is_writable};
use prelude::*;


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

pub struct SentryCliUpdateInfo<'a> {
    config: &'a Config,
    latest_release: Option<SentryCliRelease>,
}


impl<'a> SentryCliUpdateInfo<'a> {
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
            fail!("Could not get download URL for latest release.");
        }
    }

    pub fn assert_updatable(&self) -> Result<()> {
        if is_homebrew_install() {
            println!("This installation of sentry-cli is managed through homebrew");
            println!("Please use homebrew to update sentry-cli:");
            println!("");
            println!("{} brew upgrade sentry-cli", style("$").dim());
            return Err(ErrorKind::QuietExit(1).into());
        }
        if is_npm_install() {
            println!("This installation of sentry-cli is managed through npm/yarn");
            println!("Please use npm/yearn to update sentry-cli");
            return Err(ErrorKind::QuietExit(1).into());
        }
        if self.latest_release.is_none() {
            fail!("Could not get the latest release version.");
        }
        Ok(())
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
        let cfg = Config::from_cli_config()?;
        let api = Api::new(&cfg);
        match api.download_with_progress(self.download_url()?, &mut f) {
            Ok(_) => {}
            Err(err) => {
                fs::remove_file(tmp_path).ok();
                fail!(err);
            }
        };

        set_executable_mode(&tmp_path)?;
        rename_exe(&exe, &tmp_path, elevate)?;
        Ok(())
    }
}

pub fn get_latest_sentrycli_release<'a>(cfg: &'a Config) -> Result<SentryCliUpdateInfo<'a>> {
    let api = Api::new(&cfg);
    Ok(SentryCliUpdateInfo {
        config: cfg,
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
