use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{bail, Error, Result};
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};
use console::style;
use log::{debug, info};
use proguard::ProguardMapping;
use symbolic::common::ByteView;
use uuid::Uuid;

use crate::api::{Api, AssociateDsyms};
use crate::config::Config;
use crate::utils::android::{dump_proguard_uuids_as_properties, AndroidManifest};
use crate::utils::args::ArgExt;
use crate::utils::fs::{get_sha1_checksum, TempFile};
use crate::utils::system::QuietExit;
use crate::utils::ui::{copy_with_progress, make_byte_progress_bar};

#[derive(Debug)]
struct MappingRef {
    pub path: PathBuf,
    pub size: u64,
    pub uuid: Uuid,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload ProGuard mapping files to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("The path to the mapping files.")
                .multiple_values(true)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .value_name("VERSION")
                .requires("app_id")
                .help(
                    "Optionally associate the mapping files with a human \
                     readable version.{n}This helps you understand which \
                     ProGuard files go with which version of your app.",
                ),
        )
        .arg(
            Arg::new("version_code")
                .long("version-code")
                .value_name("VERSION_CODE")
                .requires("app_id")
                .requires("version")
                .help(
                    "Optionally associate the mapping files with a version \
                     code.{n}This helps you understand which ProGuard files \
                     go with which version of your app.",
                ),
        )
        .arg(
            Arg::new("app_id")
                .long("app-id")
                .value_name("APP_ID")
                .requires("version")
                .help(
                    "Optionally associate the mapping files with an application \
                     ID.{n}If you have multiple apps in one sentry project, you can \
                     then easily tell them apart.",
                ),
        )
        .arg(
            Arg::new("platform")
                .long("platform")
                .value_name("PLATFORM")
                .requires("app_id")
                .help(
                    "Optionally defines the platform for the app association. \
                     [defaults to 'android']",
                ),
        )
        .arg(
            Arg::new("no_reprocessing")
                .long("no-reprocessing")
                .help("Do not trigger reprocessing after upload."),
        )
        .arg(Arg::new("no_upload").long("no-upload").help(
            "Disable the actual upload.{n}This runs all steps for the \
             processing but does not trigger the upload (this also \
             automatically disables reprocessing).  This is useful if you \
             just want to verify the mapping files and write the \
             proguard UUIDs into a properties file.",
        ))
        .arg(
            Arg::new("android_manifest")
                .long("android-manifest")
                .value_name("PATH")
                .conflicts_with("app_id")
                .help("Read version and version code from an Android manifest file."),
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
    let api = Api::current();

    let paths: Vec<_> = match matches.get_many::<String>("paths") {
        Some(paths) => paths.collect(),
        None => {
            return Ok(());
        }
    };
    let mut mappings = vec![];
    let mut all_checksums = vec![];

    let android_manifest = if let Some(path) = matches.get_one::<String>("android_manifest") {
        Some(AndroidManifest::from_path(path)?)
    } else {
        None
    };

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
        match fs::metadata(path) {
            Ok(md) => {
                let byteview = ByteView::open(path).map_err(Error::new)?;
                let mapping = ProguardMapping::new(&byteview);
                if !mapping.has_line_info() {
                    eprintln!(
                        "warning: proguard mapping '{path}' was ignored because it \
                         does not contain any line information."
                    );
                } else {
                    let mut f = fs::File::open(path)?;
                    let sha = get_sha1_checksum(&mut f)?.to_string();
                    debug!("SHA1 for mapping file '{}': '{}'", path, sha);
                    all_checksums.push(sha);
                    mappings.push(MappingRef {
                        path: PathBuf::from(path),
                        size: md.len(),
                        uuid: forced_uuid.copied().unwrap_or_else(|| mapping.uuid()),
                    });
                }
            }
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

    if mappings.is_empty() && matches.contains_id("require_one") {
        println!();
        eprintln!("{}", style("error: found no mapping files to upload").red());
        return Err(QuietExit(1).into());
    }

    println!("{} compressing mappings", style(">").dim());
    let tf = TempFile::create()?;
    {
        let mut zip = zip::ZipWriter::new(tf.open()?);

        for mapping in &mappings {
            let pb = make_byte_progress_bar(mapping.size);
            zip.start_file(
                format!("proguard/{}.txt", mapping.uuid),
                zip::write::FileOptions::default(),
            )?;
            copy_with_progress(&pb, &mut fs::File::open(&mapping.path)?, &mut zip)?;
            pb.finish_and_clear();
        }
    }

    // write UUIDs into the mapping file.
    if let Some(p) = matches.get_one::<String>("write_properties") {
        let uuids: Vec<_> = mappings.iter().map(|x| x.uuid).collect();
        dump_proguard_uuids_as_properties(p, &uuids)?;
    }

    if matches.contains_id("no_upload") {
        println!("{} skipping upload.", style(">").dim());
        return Ok(());
    }

    println!("{} uploading mappings", style(">").dim());
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;

    info!(
        "Issuing a command for Organization: {} Project: {}",
        org, project
    );

    let rv = api.upload_dif_archive(&org, &project, tf.path())?;
    println!(
        "{} Uploaded a total of {} new mapping files",
        style(">").dim(),
        style(rv.len()).yellow()
    );
    if !rv.is_empty() {
        println!("Newly uploaded debug symbols:");
        for df in rv {
            println!("  {}", style(&df.id()).dim());
        }
    }

    // update the uuids
    if let Some(android_manifest) = android_manifest {
        api.associate_android_proguard_mappings(&org, &project, &android_manifest, all_checksums)?;

    // if values are given associate
    } else if let Some(app_id) = matches.get_one::<String>("app_id") {
        api.associate_dsyms(
            &org,
            &project,
            &AssociateDsyms {
                platform: matches
                    .get_one::<String>("platform")
                    .map(String::as_str)
                    .unwrap_or("android")
                    .to_string(),
                checksums: all_checksums,
                name: app_id.to_string(),
                app_id: app_id.to_string(),
                version: matches.get_one::<String>("version").unwrap().to_owned(),
                build: matches.get_one::<String>("version_code").cloned(),
            },
        )?;
    }

    // If wanted trigger reprocessing
    if !matches.contains_id("no_reprocessing") && !matches.contains_id("no_upload") {
        if !api.trigger_reprocessing(&org, &project)? {
            println!(
                "{} Server does not support reprocessing. Not triggering.",
                style(">").dim()
            );
        }
    } else {
        println!("{} skipped reprocessing", style(">").dim());
    }

    Ok(())
}
