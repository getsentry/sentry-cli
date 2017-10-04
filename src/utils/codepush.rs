use std::env;
use std::str;
use std::process;

use serde_json;
use console::strip_ansi_codes;
use glob::{glob_with, MatchOptions};

use prelude::*;
use utils::xcode::{InfoPlist, XcodeProjectInfo};
use utils::releases::{get_xcode_release_name, infer_gradle_release_name};

#[derive(Debug, Deserialize)]
pub struct CodePushPackage {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct CodePushDeployment {
    pub name: String,
    pub package: Option<CodePushPackage>,
}

pub fn get_codepush_deployments(app: &str)
    -> Result<Vec<CodePushDeployment>>
{
    let p = process::Command::new("code-push")
        .arg("deployment")
        .arg("ls")
        .arg(app)
        .arg("--format")
        .arg("json")
        .output()?;
    if !p.status.success() {
        let msgstr;
        let detail = if let Ok(msg) = str::from_utf8(&p.stderr) {
            msgstr = strip_ansi_codes(msg);
            if &msgstr[..9] == "[Error]  " {
                &msgstr[9..]
            } else if &msgstr[..8] == "[Error] " {
                &msgstr[8..]
            } else {
                &msgstr
            }
        } else {
            "Unknown Error"
        };
        return Err(format!("Failed to get codepush deployments ({})", detail).into());
    }
    Ok(serde_json::from_slice(&p.stdout)?)
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

    Err(format!("could not find deployment {} for {}", deployment, app).into())
}

pub fn get_codepush_release(package: &CodePushPackage, platform: &str,
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
