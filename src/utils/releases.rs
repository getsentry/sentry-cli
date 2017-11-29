use std::fs;
use std::io::Read;
use std::env;
use std::path::PathBuf;

use utils::vcs;
use utils::xcode::InfoPlist;
use utils::cordova::CordovaConfig;

use regex::Regex;

use prelude::*;


pub fn get_cordova_release_name() -> Result<Option<String>> {
    let here = env::current_dir()?;
    let path = here.join("config.xml");
    if_chain! {
        if let Ok(md) = path.metadata();
        if md.is_file();
        if let Ok(Some(config)) = CordovaConfig::load(path);
        then {
            return Ok(Some(format!("{}-{}", config.id(), config.version())));
        }
    }
    return Ok(None);
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
    // this is similar to utils::codepush::get_codepush_release
    lazy_static! {
        static ref APP_ID_RE: Regex = Regex::new(
            r#"applicationId\s+["']([^"']*)["']"#).unwrap();
        static ref VERSION_NAME_RE: Regex = Regex::new(
            r#"versionName\s+["']([^"']*)["']"#).unwrap();
    }

    let mut contents = String::new();
    let mut here = path.map_or(env::current_dir()?, |p| p.into());
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
                return Ok(Some(format!("{}-{}", &app_id_caps[1], &version_caps[1])));
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
    if let Some(release) = get_cordova_release_name()? {
        return Ok(release);
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
        Err("Could not automatically determine release name".into())
    }
}
