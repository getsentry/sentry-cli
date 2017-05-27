use std::fs;
use std::str;
use std::process;

use serde_json;
use console::strip_ansi_codes;
use glob::{glob, glob_with, MatchOptions};
use elementtree::Element;

use prelude::*;
use xcode::InfoPlist;


#[derive(Debug, Deserialize)]
pub struct CodePushPackage {
    #[serde(rename="appVersion")]
    pub app_version: String,
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

pub fn get_codepush_release(package: &CodePushPackage, platform: &str)
    -> Result<String>
{
    if platform == "ios" {
        let mut opts = MatchOptions::new();
        opts.case_sensitive = false;
        for entry_rv in glob_with("ios/*/info.plist", &opts)? {
            if_chain! {
                if let Ok(entry) = entry_rv;
                if let Some(base) = entry.parent();
                if let Some(folder_os) = base.file_name();
                if let Some(folder) = folder_os.to_str();
                if !folder.ends_with("-tvOS");
                if let Ok(md) = entry.metadata();
                if md.is_file();
                then {
                    let plist = InfoPlist::from_path(&entry)?;
                    return Ok(format!("{}-{}:{}",
                                      plist.derived_bundle_id(folder),
                                      package.app_version,
                                      package.label));
                }
            }
        }
        return Err("Could not find plist".into());
    } else if platform == "android" {
        for entry_rv in glob("android/app/**/AndroidManifest.xml")? {
            if_chain! {
                if let Ok(entry) = entry_rv;
                if let Ok(md) = entry.metadata();
                if md.is_file();
                then {
                    let f = fs::File::open(entry)?;
                    let manifest = Element::from_reader(f)?;
                    let id = manifest.get_attr("package")
                        .ok_or_else(|| Error::from(
                            "Could not find package in android manifest"))?;
                    return Ok(format!("{}-{}:{}",
                                      id,
                                      package.app_version,
                                      package.label));
                }
            }
        }
        return Err("Could not find AndroidManifest.xml".into());
    }
    return Err(format!("Unsupported platform '{}'", platform).into());
}
