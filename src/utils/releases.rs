use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use if_chain::if_chain;
use lazy_static::lazy_static;
use regex::Regex;

use crate::utils::cordova::CordovaConfig;
use crate::utils::vcs;
use crate::utils::xcode::InfoPlist;

pub fn get_cordova_release_name(path: Option<PathBuf>) -> Result<Option<String>> {
    let here = path.unwrap_or(env::current_dir()?);
    let platform = match here.file_name().and_then(OsStr::to_str) {
        Some("android") => "android",
        Some("ios") => "ios",
        _ => return Ok(None),
    };
    let base = match here.parent().and_then(Path::parent) {
        Some(path) => path,
        None => return Ok(None),
    };

    let path = base.join("config.xml");
    if_chain! {
        if let Ok(md) = path.metadata();
        if md.is_file();
        if let Ok(Some(config)) = CordovaConfig::load(path);
        then {
            match platform {
                "android" => Ok(Some(config.android_release_name())),
                "ios" => Ok(Some(config.ios_release_name())),
                _ => unreachable!(),
            }
        } else {
            Ok(None)
        }
    }
}

pub fn get_xcode_release_name(plist: Option<InfoPlist>) -> Result<Option<String>> {
    // if we are executed from within xcode, then we can use the environment
    // based discovery to get a release name without any interpolation.
    if let Some(plist) = plist.or(InfoPlist::discover_from_env()?) {
        return Ok(Some(plist.get_release_name()));
    }

    Ok(None)
}

pub fn infer_gradle_release_name(path: Option<PathBuf>) -> Result<Option<String>> {
    lazy_static! {
        static ref APP_ID_RE: Regex = Regex::new(r#"applicationId\s+["']([^"']*)["']"#).unwrap();
        static ref VERSION_NAME_RE: Regex =
            Regex::new(r#"versionName\s+["']([^"']*)["']"#).unwrap();
    }

    let mut contents = String::new();
    let mut here = path.unwrap_or(env::current_dir()?);
    loop {
        if_chain! {
            if let Ok(build_md) = here.join("build.gradle").metadata();
            if build_md.is_file();
            if let Ok(app_md) = here.join("app/build.gradle").metadata();
            if app_md.is_file();
            if let Ok(mut f) = fs::File::open(here.join("app/build.gradle"));
            if f.read_to_string(&mut contents).is_ok();
            if let Some(app_id_caps) = APP_ID_RE.captures(&contents);
            if let Some(version_caps) = VERSION_NAME_RE.captures(&contents);
            then {
                return Ok(Some(format!("{}@{}", &app_id_caps[1], &version_caps[1])));
            }
        }
        if !here.pop() {
            break;
        }
    }

    Ok(None)
}

/// Detects the release name for the current working directory.
pub fn detect_release_name() -> Result<String> {
    // cordova release detection first.
    if let Some(release) = get_cordova_release_name(None)? {
        return Ok(release);
    }

    // try Heroku #1 https://devcenter.heroku.com/changelog-items/630
    if let Ok(release) = env::var("SOURCE_VERSION") {
        if !release.is_empty() {
            return Ok(release);
        }
    }

    // try Heroku #2: https://docs.sentry.io/product/integrations/deployment/heroku/#configure-releases
    if let Ok(release) = env::var("HEROKU_SLUG_COMMIT") {
        if !release.is_empty() {
            return Ok(release);
        }
    }

    // try AWS CodeBuild: https://docs.aws.amazon.com/codebuild/latest/userguide/build-env-ref-env-vars.html
    if let Ok(release) = env::var("CODEBUILD_RESOLVED_SOURCE_VERSION") {
        if !release.is_empty() {
            return Ok(release);
        }
    }

    // try CircleCI: https://circleci.com/docs/2.0/env-vars/
    if let Ok(release) = env::var("CIRCLE_SHA1") {
        if !release.is_empty() {
            return Ok(release);
        }
    }

    // try Cloudflare Pages: https://developers.cloudflare.com/pages/platform/build-configuration/#environment-variables
    if let Ok(release) = env::var("CF_PAGES_COMMIT_SHA") {
        if !release.is_empty() {
            return Ok(release);
        }
    }

    // for now only execute this on macs.  The reason is that this uses
    // xcodebuild which does not exist anywhere but there.
    if_chain! {
        if cfg!(target_os="macos");
        if let Some(release) = get_xcode_release_name(None)?;
        then {
            return Ok(release)
        }
    }

    // For android we badly parse gradle files.  We do this because most of the
    // time now people set the ids and versions in the gradle files instead of
    // the xml manifests.
    if let Some(release) = infer_gradle_release_name(None)? {
        return Ok(release);
    }

    if let Ok(head) = vcs::find_head() {
        Ok(head)
    } else {
        bail!("Could not automatically determine release name");
    }
}
