use std::env;
use std::io;
use std::path::Path;
use std::process;
use std::str;

use anyhow::{bail, format_err, Error, Result};
use console::strip_ansi_codes;
use glob::{glob_with, MatchOptions};
use if_chain::if_chain;
use serde::Deserialize;

use crate::utils::releases::{get_xcode_release_name, infer_gradle_release_name};
use crate::utils::xcode::{InfoPlist, XcodeProjectInfo};

#[cfg(not(windows))]
static CODEPUSH_BIN_PATH: &str = "code-push";
#[cfg(not(windows))]
static CODEPUSH_NPM_PATH: &str = "node_modules/.bin/code-push";

#[cfg(windows)]
static CODEPUSH_BIN_PATH: &str = "code-push.cmd";
#[cfg(windows)]
static CODEPUSH_NPM_PATH: &str = "node_modules/.bin/code-push.cmd";

#[derive(Debug, Deserialize)]
pub struct CodePushPackage {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct CodePushDeployment {
    pub name: String,
    pub package: Option<CodePushPackage>,
}

fn get_codepush_error(output: &process::Output) -> Error {
    if let Ok(message) = str::from_utf8(&output.stderr) {
        let stripped = strip_ansi_codes(message);
        format_err!(if let Some(rest) = stripped.strip_prefix("[Error]  ") {
            rest
        } else if let Some(rest) = stripped.strip_prefix("[Error] ") {
            rest
        } else {
            &stripped
        }
        .to_string(),)
    } else {
        format_err!("Unknown Error")
    }
}

pub fn get_codepush_deployments(app: &str) -> Result<Vec<CodePushDeployment>> {
    let codepush_bin = if Path::new(CODEPUSH_NPM_PATH).exists() {
        CODEPUSH_NPM_PATH
    } else {
        CODEPUSH_BIN_PATH
    };

    let output = process::Command::new(codepush_bin)
        .arg("deployment")
        .arg("ls")
        .arg(app)
        .arg("--format")
        .arg("json")
        .output()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                Error::msg("Codepush not found. Is it installed and configured on the PATH?")
            } else {
                Error::from(e).context("Failed to run codepush")
            }
        })?;

    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            let format_err = format!("Command `{codepush_bin} deployment ls {app} --format json` failed to produce a valid JSON output.");
            panic!("{}", format_err);
        }))
    } else {
        Err(get_codepush_error(&output).context("Failed to get codepush deployments"))
    }
}

pub fn get_codepush_package(app: &str, deployment: &str) -> Result<CodePushPackage> {
    let deployments = get_codepush_deployments(app)?;
    for dep in deployments {
        if_chain! {
            if dep.name == deployment;
            if let Some(pkg) = dep.package;
            then {
                return Ok(pkg);
            }
        }
    }

    bail!("Could not find deployment {} for {}", deployment, app)
}

pub fn get_react_native_codepush_release(
    package: &CodePushPackage,
    platform: &str,
    bundle_id_override: Option<&str>,
) -> Result<String> {
    if let Some(bundle_id) = bundle_id_override {
        return Ok(format!("{}+codepush:{}", bundle_id, package.label));
    }

    if platform == "ios" {
        if !cfg!(target_os = "macos") {
            bail!("Codepush releases for iOS require OS X if no bundle ID is specified");
        }
        let mut opts = MatchOptions::new();
        opts.case_sensitive = false;
        for entry in (glob_with("ios/*.xcodeproj", opts)?).flatten() {
            let pi = XcodeProjectInfo::from_path(entry)?;
            if let Some(ipl) = InfoPlist::from_project_info(&pi)? {
                if let Some(release_name) = get_xcode_release_name(Some(ipl))? {
                    return Ok(format!("{}+codepush:{}", release_name, package.label));
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
                    return Ok(format!("{}+codepush:{}", release_name, package.label));
                } else {
                    bail!("Could not parse app id from build.gradle");
                }
            }
        }
        bail!("Could not find AndroidManifest.xml");
    }
    bail!("Unsupported platform '{}'", platform);
}
