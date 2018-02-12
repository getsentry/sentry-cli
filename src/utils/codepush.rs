use std::env;
use std::io;
use std::path::Path;
use std::process;
use std::str;

use console::strip_ansi_codes;
use glob::{glob_with, MatchOptions};
use serde_json;

use prelude::*;
use utils::releases::{get_xcode_release_name, infer_gradle_release_name};
use utils::xcode::{InfoPlist, XcodeProjectInfo};

static CODEPUSH_BIN_PATH: &'static str = "code-push";
static CODEPUSH_NPM_PATH: &'static str = "node_modules/.bin/code-push";

#[derive(Debug, Deserialize)]
pub struct CodePushPackage {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct CodePushDeployment {
    pub name: String,
    pub package: Option<CodePushPackage>,
}

fn get_codepush_error(output: process::Output) -> Error {
    if let Ok(message) = str::from_utf8(&output.stderr) {
        let stripped = strip_ansi_codes(message);
        Error::from(if stripped.starts_with("[Error]  ") {
            &stripped[9..]
        } else if stripped.starts_with("[Error] ") {
            &stripped[8..]
        } else {
            &stripped
        })
    } else {
        Error::from("Unknown Error")
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
                "Codepush not found. Is it installed and configured on the PATH?".into()
            } else {
                Error::from(e).chain_err(|| "Failed to run codepush")
            }
        })?;

    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout)?)
    } else {
        Err(get_codepush_error(output)).chain_err(|| "Failed to get codepush deployments")
    }
}

pub fn get_codepush_package(app: &str, deployment: &str)
    -> Result<CodePushPackage>
{
    let deployments = get_codepush_deployments(app)?;
    for dep in deployments {
        if_chain! {
            if &dep.name == deployment;
            if let Some(pkg) = dep.package;
            then {
                return Ok(pkg);
            }
        }
    }

    Err(format!("Could not find deployment {} for {}", deployment, app).into())
}

pub fn get_react_native_codepush_release(package: &CodePushPackage, platform: &str,
                                         bundle_id_override: Option<&str>)
    -> Result<String>
{
    if let Some(bundle_id) = bundle_id_override {
        return Ok(format!("{}-codepush:{}", bundle_id, package.label));
    }

    if platform == "ios" {
        if !cfg!(target_os="macos") {
            return Err("Codepush releases for iOS require OS X if no \
                        bundle ID is specified".into());
        }
        let mut opts = MatchOptions::new();
        opts.case_sensitive = false;
        for entry_rv in glob_with("ios/*.xcodeproj", &opts)? {
            if let Ok(entry) = entry_rv {
                let pi = XcodeProjectInfo::from_path(&entry)?;
                if let Some(ipl) = InfoPlist::from_project_info(&pi)? {
                    if let Some(release_name) = get_xcode_release_name(Some(ipl))? {
                        return Ok(format!("{}-codepush:{}", release_name, package.label));
                    }
                }
            }
        }
        return Err("Could not find plist".into());
    } else if platform == "android" {
        if_chain! {
            if let Ok(here) = env::current_dir();
            if let Ok(android_folder) = here.join("android").metadata();
            if android_folder.is_dir();
            then {
                return if let Some(release_name) = infer_gradle_release_name(Some(here.join("android")))? {
                    Ok(format!("{}-codepush:{}", release_name, package.label))
                } else {
                    Err("Could not parse app id from build.gradle".into())
                }
            }
        }
        return Err("Could not find AndroidManifest.xml".into());
    }
    return Err(format!("Unsupported platform '{}'", platform).into());
}
