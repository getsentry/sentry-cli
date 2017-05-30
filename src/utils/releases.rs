use std::fs;
use std::io::Read;
use std::env;
use std::path::Path;

use utils::vcs;
use utils::xcode::{InfoPlist, XcodeProjectInfo};

use regex::Regex;

use prelude::*;


fn get_xcode_project_info(path: &Path) -> Result<Option<XcodeProjectInfo>> {
    if_chain! {
        if let Some(filename_os) = path.file_name();
        if let Some(filename) = filename_os.to_str();
        if filename.ends_with(".xcodeproj");
        then {
            return Ok(Some(XcodeProjectInfo::from_path(path)?));
        }
    }

    let mut projects = vec![];
    for entry_rv in fs::read_dir(path)? {
        if let Ok(entry) = entry_rv {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".xcodeproj") {
                    projects.push(entry.path().to_path_buf());
                }
            }
        }
    }

    if projects.len() == 1 {
        Ok(Some(XcodeProjectInfo::from_path(&projects[0])?))
    } else {
        Ok(None)
    }
}

fn get_xcode_release_name() -> Result<Option<String>> {
    // if we are executed from within xcode, then we can use the environment
    // based discovery to get a release name without any interpolation.
    if let Some(plist) = InfoPlist::discover_from_env()? {
        return Ok(Some(format!("{}-{}", plist.bundle_id(), plist.version())));
    }

    // Otherwise look upwards for the most likely root of the project.  In
    // that case because we lack information to actually assemble the real
    // release name we will try to determin it from some well known folder names.
    if let Ok(mut here) = env::current_dir() {
        loop {
            if_chain! {
                if let Some(pi) = get_xcode_project_info(&here)?;
                if let Some(config) = pi.get_configuration("release")
                    .or_else(|| pi.get_configuration("debug"));
                if let Some(target) = pi.get_first_target();
                then {
                    return Ok(Some(pi.get_release_name(target, config, None)?));
                }
            }
            if !here.pop() {
                break;
            }
        }
    }

    Ok(None)
}

fn infer_gradle_release_name() -> Result<Option<String>> {
    // this is similar to utils::codepush::get_codepush_release
    lazy_static! {
        static ref APP_ID_RE: Regex = Regex::new(
            r#"applicationId\s+["']([^"']*)["']"#).unwrap();
        static ref VERSION_NAME_RE: Regex = Regex::new(
            r#"versionName\s+["']([^"']*)["']"#).unwrap();
    }

    let mut contents = String::new();
    if let Ok(mut here) = env::current_dir() {
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
    }

    Ok(None)
}

/// Detects the release name for the current working directory.
pub fn detect_release_name() -> Result<String> {
    // for now only execute this on macs.  The reason is that this uses
    // xcodebuild which does not exist anywhere but there.
    if_chain! {
        if cfg!(target_os="macos");
        if let Some(release) = get_xcode_release_name()?;
        then {
            return Ok(release)
        }
    }

    // For android we badly parse gradle files.  We do this because most of the
    // time now people set the ids and versions in the gradle files instead of
    // the xml manifests.
    if let Some(release) = infer_gradle_release_name()? {
        return Ok(release);
    }

    if let Ok(head) = vcs::find_head() {
        Ok(head)
    } else {
        Err("Could not automatically determine release name".into())
    }
}
