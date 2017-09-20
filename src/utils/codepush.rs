use std::fs;
use std::io::Read;
use std::str;
use std::process;

use serde_json;
use console::strip_ansi_codes;
use glob::{glob, glob_with, MatchOptions};
use regex::Regex;

use prelude::*;
use utils::xcode::{InfoPlist, XcodeProjectInfo};


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
    // this is similar to utils::releases::infer_gradle_release_name
    lazy_static! {
        static ref APP_ID_RE: Regex = Regex::new(
            r#"applicationId\s+["']([^"']*)["']"#).unwrap();
    }

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
                    return Ok(format!("{}-codepush:{}", ipl.get_release_name(), package.label));
                }
            }
        }
        return Err("Could not find plist".into());
    } else if platform == "android" {
        for entry_rv in glob("android/app/build.gradle")? {
            let mut s = String::new();
            if_chain! {
                if let Ok(entry) = entry_rv;
                if let Ok(md) = entry.metadata();
                if md.is_file();
                if let Ok(mut f) = fs::File::open(entry);
                if f.read_to_string(&mut s).is_ok();
                then {
                    return if let Some(app_id_caps) = APP_ID_RE.captures(&s) {
                        Ok(format!("{}-codepush:{}", &app_id_caps[1], package.label))
                    } else {
                        Err("Could not parse app id from build.gradle".into())
                    };
                }
            }
        }
        return Err("Could not find AndroidManifest.xml".into());
    }
    return Err(format!("Unsupported platform '{}'", platform).into());
}
