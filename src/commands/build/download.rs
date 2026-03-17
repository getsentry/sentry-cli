use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use log::info;

use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt as _;
use crate::utils::fs::TempFile;

pub fn make_command(command: Command) -> Command {
    command
        .about("Download a build from a project.")
        .long_about("Download a build from a project.\n\nThis feature only works with Sentry SaaS.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("build_id")
                .long("build-id")
                .short('b')
                .required(true)
                .help("The ID of the build to download."),
        )
        .arg(Arg::new("output").long("output").help(
            "The output file path. Defaults to \
                    'preprod_artifact_<build_id>.<ext>' in the current directory, \
                    where ext is ipa or apk depending on the platform.",
        ))
}

/// For iOS builds, the install URL points to a plist manifest.
/// Replace the response_format to download the actual IPA binary instead.
fn ensure_binary_format(url: &str) -> String {
    url.replace("response_format=plist", "response_format=ipa")
}

/// Extract the file extension from the response_format query parameter.
fn extension_from_url(url: &str) -> Result<&str> {
    if url.contains("response_format=ipa") {
        Ok("ipa")
    } else if url.contains("response_format=apk") {
        Ok("apk")
    } else {
        bail!("Unsupported build format in download URL.")
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let build_id = matches.get_one::<String>("build_id").unwrap();

    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    info!("Fetching install details for build {build_id}");
    let details = authenticated_api.get_build_install_details(&org, build_id)?;

    if !details.is_installable {
        bail!("Build {build_id} is not installable.");
    }

    let install_url = details
        .install_url
        .ok_or_else(|| anyhow::anyhow!("Build {build_id} has no install URL."))?;

    let download_url = ensure_binary_format(&install_url);

    let output_path = match matches.get_one::<String>("output") {
        Some(path) => PathBuf::from(path),
        None => {
            let ext = extension_from_url(&download_url)?;
            PathBuf::from(format!("preprod_artifact_{build_id}.{ext}"))
        }
    };

    info!("Downloading build {build_id} to {}", output_path.display());

    let tmp = TempFile::create()?;
    let mut file = tmp.open()?;
    let response = authenticated_api.download_installable_build(&download_url, &mut file)?;

    if response.failed() {
        bail!(
            "Failed to download build (server returned status {}).",
            response.status()
        );
    }

    drop(file);
    fs::copy(tmp.path(), &output_path)?;

    println!("Successfully downloaded build to {}", output_path.display());

    Ok(())
}
