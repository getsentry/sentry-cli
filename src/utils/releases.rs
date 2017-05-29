use std::fs;
use std::env;
use std::path::Path;

use utils::vcs;
use utils::xcode::{InfoPlist, XcodeProjectInfo};

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

fn get_xcode_release_name() -> Result<Option<String>>
{
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
                    return Ok(Some(pi.get_release_name(target, config)?));
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
pub fn detect_release_name() -> Result<String>
{
    // for now only execute this on macs.  The reason is that this uses
    // xcodebuild which does not exist anywhere but there.
    if_chain! {
        if cfg!(target_os="macos");
        if let Some(release) = get_xcode_release_name()?;
        then {
            return Ok(release)
        }
    }

    Ok(vcs::find_head()?)
}
