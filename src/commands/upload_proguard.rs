use std::io;

use anyhow::{bail, Error, Result};
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use console::style;
use symbolic::common::ByteView;
use uuid::Uuid;

use crate::api::Api;
use crate::config::Config;
use crate::utils::android::dump_proguard_uuids_as_properties;
use crate::utils::args::ArgExt as _;
use crate::utils::proguard;
use crate::utils::proguard::ProguardMapping;
use crate::utils::system::QuietExit;

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload ProGuard mapping files to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("The path to the mapping files.")
                .num_args(1..)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("no_upload")
                .long("no-upload")
                .action(ArgAction::SetTrue)
                .help(
                    "Disable the actual upload.{n}This runs all steps for the \
                    processing but does not trigger the upload.  This is useful if you \
                    just want to verify the mapping files and write the \
                    proguard UUIDs into a properties file.",
                ),
        )
        .arg(
            Arg::new("write_properties")
                .long("write-properties")
                .value_name("PATH")
                .help(
                    "Write the UUIDs for the processed mapping files into \
                     the given properties file.",
                ),
        )
        .arg(
            Arg::new("require_one")
                .long("require-one")
                .action(ArgAction::SetTrue)
                .help("Requires at least one file to upload or the command will error."),
        )
        .arg(
            Arg::new("uuid")
                .long("uuid")
                .short('u')
                .value_name("UUID")
                .value_parser(Uuid::parse_str)
                .help(
                    "Explicitly override the UUID of the mapping file with another one.{n}\
                     This should be used with caution as it means that you can upload \
                     multiple mapping files if you don't take care.  This however can \
                     be useful if you have a build process in which you need to know \
                     the UUID of the proguard file before it was created.  If you upload \
                     a file with a forced UUID you can only upload a single proguard file.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let paths: Vec<_> = match matches.get_many::<String>("paths") {
        Some(paths) => paths.collect(),
        None => {
            return Ok(());
        }
    };
    let mut mappings = vec![];

    let forced_uuid = matches.get_one::<Uuid>("uuid");
    if forced_uuid.is_some() && paths.len() != 1 {
        bail!(
            "When forcing a UUID a single proguard file needs to be \
             provided, got {}",
            paths.len()
        );
    }

    // since the mappings are quite small we don't bother doing a second http
    // request to figure out if any of the checksums are missing.  We just ship
    // them all up.
    for path in &paths {
        match ByteView::open(path) {
            Ok(byteview) => match ProguardMapping::try_from(byteview) {
                Ok(mapping) => mappings.push(mapping),
                Err(e) => eprintln!("warning: ignoring proguard mapping '{path}': {e}"),
            },
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                eprintln!(
                    "warning: proguard mapping '{path}' does not exist. This \
                     might be because the build process did not generate \
                     one (for instance because -dontobfuscate is used)"
                );
            }
            Err(err) => {
                return Err(
                    Error::from(err).context(format!("failed to open proguard mapping '{path}'"))
                );
            }
        }
    }

    if let Some(&uuid) = forced_uuid {
        // There should only be one mapping if we are forcing a UUID.
        // This is checked earlier.
        for mapping in &mut mappings {
            mapping.force_uuid(uuid);
        }
    }

    // We are done constructing the mappings, redeclare as immutable.
    let mappings = mappings;

    let api = Api::current();
    let config = Config::current();

    if mappings.is_empty() && matches.get_flag("require_one") {
        println!();
        eprintln!("{}", style("error: found no mapping files to upload").red());
        return Err(QuietExit(1).into());
    }

    // write UUIDs into the mapping file.
    if let Some(p) = matches.get_one::<String>("write_properties") {
        let uuids: Vec<_> = mappings.iter().map(|x| x.uuid()).collect();
        dump_proguard_uuids_as_properties(p, &uuids)?;
    }

    if matches.get_flag("no_upload") {
        println!("{} skipping upload.", style(">").dim());
        return Ok(());
    }

    let authenticated_api = api.authenticated()?;
    let (org, project) = config.get_org_and_project(matches)?;

    let chunk_upload_options = authenticated_api.get_chunk_upload_options(&org)?;
    proguard::chunk_upload(&mappings, chunk_upload_options, &org, &project)
}
