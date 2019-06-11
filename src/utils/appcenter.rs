use std::env;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{Command, Output};
use std::str;

use console::strip_ansi_codes;
use failure::{bail, err_msg, Error};
use glob::{glob_with, MatchOptions};
// use serde::de::{Deserialize, Deserializer, Error as DeError};
use if_chain::if_chain;
use serde::de;

use crate::utils::releases::{get_xcode_release_name, infer_gradle_release_name};
use crate::utils::xcode::{InfoPlist, XcodeProjectInfo};

#[cfg(not(windows))]
static APPCENTER_BIN_PATH: &str = "appcenter";
#[cfg(not(windows))]
static APPCENTER_NPM_PATH: &str = "node_modules/.bin/appcenter";

#[cfg(windows)]
static APPCENTER_BIN_PATH: &str = "appcenter.cmd";
#[cfg(windows)]
static APPCENTER_NPM_PATH: &str = "node_modules/.bin/appcenter.cmd";

static APPCENTER_NOT_FOUND: &str = "AppCenter CLI not found

Install with `npm install -g appcenter-cli` and make sure it is on the PATH.";

#[derive(Debug)]
pub struct AppCenterPackage {
    pub label: String,
}

impl<'de> de::Deserialize<'de> for AppCenterPackage {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PackageVisitor;

        impl<'de> de::Visitor<'de> for PackageVisitor {
            type Value = AppCenterPackage;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a deployment history entry")
            }

            fn visit_seq<S: de::SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> Result<AppCenterPackage, S::Error> {
                // Since we only need the package label, we can deserialize the JSON string very
                // efficiently by only looking at the first element.
                let label = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("missing package label"))?;

                // Drain the sequence, ignoring all other values.
                while seq.next_element::<de::IgnoredAny>()?.is_some() {}

                Ok(AppCenterPackage { label })
            }
        }

        deserializer.deserialize_seq(PackageVisitor)
    }
}

pub fn get_appcenter_error(output: &Output) -> Error {
    let message = match str::from_utf8(&output.stdout) {
        Ok(message) => message,
        Err(_) => "Unknown AppCenter error",
    };

    let stripped = strip_ansi_codes(message);
    let cause = if stripped.starts_with("Error: ") {
        &stripped[7..]
    } else {
        &stripped
    }
    .to_string();

    err_msg(cause)
}

pub fn get_appcenter_deployment_history(
    app: &str,
    deployment: &str,
) -> Result<Vec<AppCenterPackage>, Error> {
    let appcenter_bin = if Path::new(APPCENTER_NPM_PATH).exists() {
        APPCENTER_NPM_PATH
    } else {
        APPCENTER_BIN_PATH
    };

    let output = Command::new(appcenter_bin)
        .arg("codepush")
        .arg("deployment")
        .arg("history")
        .arg(deployment)
        .arg("--app")
        .arg(app)
        .arg("--output")
        .arg("json")
        .output()
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => APPCENTER_NOT_FOUND.into(),
            _ => Error::from(e).context("Failed to run AppCenter CLI"),
        })?;

    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout)?)
    } else {
        Err(get_appcenter_error(&output)
            .context("Failed to load AppCenter deployment history")
            .into())
    }
}

pub fn get_appcenter_package(app: &str, deployment: &str) -> Result<AppCenterPackage, Error> {
    let history = get_appcenter_deployment_history(app, deployment)?;
    if let Some(latest) = history.into_iter().last() {
        Ok(latest)
    } else {
        bail!("Could not find deployment {} for {}", deployment, app);
    }
}

pub fn get_react_native_appcenter_release(
    package: &AppCenterPackage,
    platform: &str,
    bundle_id_override: Option<&str>,
    version_name_override: Option<&str>,
) -> Result<String, Error> {
    let bundle_id_ovrr = bundle_id_override.unwrap_or("");
    let version_name_ovrr = version_name_override.unwrap_or("");

    if bundle_id_ovrr != "" && version_name_ovrr != "" {
        return Ok(format!(
            "{}-{}-codepush:{}",
            bundle_id_ovrr, version_name_ovrr, package.label
        ));
    }

    if platform == "ios" {
        if !cfg!(target_os = "macos") {
            bail!("AppCenter codepush releases for iOS require macOS if no bundle ID is specified");
        }

        let mut opts = MatchOptions::new();
        opts.case_sensitive = false;

        for entry_rv in glob_with("ios/*.xcodeproj", opts)? {
            if let Ok(entry) = entry_rv {
                let pi = XcodeProjectInfo::from_path(&entry)?;
                if let Some(ipl) = InfoPlist::from_project_info(&pi)? {
                    if let Some(release_name) = get_xcode_release_name(Some(ipl))? {
                        let vec: Vec<&str> = release_name.split('-').collect();
                        let bundle_id = if bundle_id_ovrr == "" {
                            vec[0]
                        } else {
                            bundle_id_ovrr
                        };
                        let version_name = if version_name_ovrr == "" {
                            vec[1]
                        } else {
                            version_name_ovrr
                        };
                        return Ok(format!(
                            "{}-{}-codepush:{}",
                            bundle_id, version_name, package.label
                        ));
                    }
                }
            }
        }

        bail!("Could not find plist");
    } else if platform == "android" {
        if_chain! {
            if let Ok(here) = env::current_dir();
            if let Ok(android_folder) = here.join("android").metadata();
            if android_folder.is_dir();
            then {
                if let Some(release_name) = infer_gradle_release_name(Some(here.join("android")))? {
                    let vec: Vec<&str> = release_name.split('-').collect();
                    let bundle_id = if bundle_id_ovrr == "" { vec[0] } else { bundle_id_ovrr };
                    let version_name = if version_name_ovrr == "" { vec[1] } else { version_name_ovrr };
                    return Ok(format!("{}-{}-codepush:{}", bundle_id, version_name, package.label));
                } else {
                    bail!("Could not parse app id from build.gradle");
                }
            }
        }

        bail!("Could not find AndroidManifest.xml");
    }

    bail!("Unsupported platform '{}'", platform);
}
