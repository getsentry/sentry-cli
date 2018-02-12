//! Implements a command for uploading proguard mapping files.
use std::fs;
use std::io;
use std::path::PathBuf;

use clap::{App, Arg, ArgMatches};
use console::style;
use symbolic_common::ByteView;
use symbolic_proguard::ProguardMappingView;
use uuid::Uuid;
use zip;

use api::{Api, AssociateDsyms};
use config::Config;
use errors::{Error, ErrorKind, Result, ResultExt};
use utils::android::{dump_proguard_uuids_as_properties, AndroidManifest};
use utils::args::{validate_uuid, ArgExt};
use utils::fs::{TempFile, get_sha1_checksum};
use utils::ui::{copy_with_progress, make_byte_progress_bar};

#[derive(Debug)]
struct MappingRef {
    pub path: PathBuf,
    pub size: u64,
    pub uuid: Uuid,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload ProGuard mapping files to a project.")
        .org_project_args()
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("The path to the mapping files.")
            .multiple(true)
            .number_of_values(1)
            .index(1))
        .arg(Arg::with_name("version")
             .long("version")
             .value_name("VERSION")
             .requires("app_id")
             .help("Optionally associate the mapping files with a human \
                    readable version.{n}This helps you understand which \
                    ProGuard files go with which version of your app."))
        .arg(Arg::with_name("version_code")
             .long("version-code")
             .value_name("VERSION_CODE")
             .requires("app_id")
             .requires("version")
             .help("Optionally associate the mapping files with a version \
                    code.{n}This helps you understand which ProGuard files \
                    go with which version of your app."))
        .arg(Arg::with_name("app_id")
             .long("app-id")
             .value_name("APP_ID")
             .requires("version")
             .help("Optionally associate the mapping files with an application \
                    ID.{n}If you have multiple apps in one sentry project you can \
                    then easlier tell them apart."))
        .arg(Arg::with_name("platform")
             .long("platform")
             .value_name("PLATFORM")
             .requires("app_id")
             .help("Optionally defines the platform for the app association. \
                    [defaults to 'android']"))
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Do not trigger reprocessing after upload."))
        .arg(Arg::with_name("no_upload")
             .long("no-upload")
             .help("Disable the actual upload.{n}This runs all steps for the \
                    processing but does not trigger the upload (this also \
                    automatically disables reprocessing.  This is useful if you \
                    just want to verify the mapping files and write the \
                    proguard UUIDs into a proeprties file."))
        .arg(Arg::with_name("android_manifest")
             .long("android-manifest")
             .value_name("PATH")
             .conflicts_with("app_id")
             .help("Read version and version code from an Android manifest file."))
        .arg(Arg::with_name("write_properties")
             .long("write-properties")
             .value_name("PATH")
             .help("Write the UUIDs for the processed mapping files into \
                    the given properties file."))
        .arg(Arg::with_name("require_one")
             .long("require-one")
             .help("Requires at least one file to upload or the command will error."))
        .arg(Arg::with_name("uuid")
             .long("uuid")
             .short("u")
             .value_name("UUID")
             .validator(validate_uuid)
             .help("Explicitly override the UUID of the mapping file with another one.{n}\
                    This should be used with caution as it means that you can upload \
                    multiple mapping files if you don't take care.  This however can \
                    be useful if you have a build process in which you need to know \
                    the UUID of the proguard file before it was created.  If you upload \
                    a file with a forced UUID you can only upload a single proguard file."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    let api = Api::new();

    let paths: Vec<_> = match matches.values_of("paths") {
        Some(paths) => paths.collect(),
        None => { return Ok(()); }
    };
    let mut mappings = vec![];
    let mut all_checksums = vec![];

    let android_manifest = if let Some(path) = matches.value_of("android_manifest") {
        Some(AndroidManifest::from_path(path)?)
    } else {
        None
    };

    let forced_uuid = matches.value_of("uuid").map(|x| x.parse::<Uuid>().unwrap());
    if forced_uuid.is_some() && paths.len() != 1 {
        fail!("When forcing a UUID a single proguard file needs to be \
               provided, got {}", paths.len());
    }

    // since the mappings are quite small we don't bother doing a second http
    // request to figure out if any of the checksums are missing.  We just ship
    // them all up.
    for path in &paths {
        match fs::metadata(path) {
            Ok(md) => {
                let mapping = ProguardMappingView::parse(ByteView::from_path(path)?)?;
                if !mapping.has_line_info() {
                    println_stderr!("warning: proguard mapping '{}' was ignored because it \
                                     does not contain any line information.", path);
                } else {
                    let mut f = fs::File::open(path)?;
                    all_checksums.push(get_sha1_checksum(&mut f)?.to_string());
                    mappings.push(MappingRef {
                        path: PathBuf::from(path),
                        size: md.len(),
                        uuid: forced_uuid.clone().unwrap_or_else(|| mapping.uuid()),
                    });
                }
            }
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                println_stderr!("warning: proguard mapping '{}' does not exist. This \
                                 might be because the build process did not generate \
                                 one (for instance because -dontobfuscate is used)",
                                 path);
            }
            Err(err) => {
                return Err(err).chain_err(|| Error::from(
                    format!("failed to open proguard mapping '{}'", path)))?;
            }
        }
    }

    if mappings.is_empty() && matches.is_present("require_one") {
        println!("");
        println_stderr!("{}", style("error: found no mapping files to upload").red());
        return Err(ErrorKind::QuietExit(1).into());
    }

    println!("{} compressing mappings", style(">").dim());
    let tf = TempFile::new()?;

    // add a scope here so we will flush before uploading
    {
        let mut zip = zip::ZipWriter::new(tf.open());
        for mapping in &mappings {
            let pb = make_byte_progress_bar(mapping.size);
            zip.start_file(format!("proguard/{}.txt", mapping.uuid),
                           zip::write::FileOptions::default())?;
            copy_with_progress(&pb, &mut fs::File::open(&mapping.path)?, &mut zip)?;
            pb.finish_and_clear();
        }
    }

    // write UUIDs into the mapping file.
    if let Some(p) = matches.value_of("write_properties") {
        let uuids: Vec<_> = mappings.iter().map(|x| x.uuid).collect();
        dump_proguard_uuids_as_properties(p, &uuids)?;
    }

    if matches.is_present("no_upload") {
        println!("{} skipping upload.", style(">").dim());
        return Ok(());
    }

    println!("{} uploading mappings", style(">").dim());
    let config = Config::get_current();
    let (org, project) = config.get_org_and_project(matches)?;
    let rv = api.upload_dif_archive(&org, &project, tf.path())?;
    println!("{} Uploaded a total of {} new mapping files",
             style(">").dim(), style(rv.len()).yellow());
    if rv.len() > 0 {
        println!("Newly uploaded debug symbols:");
        for df in rv {
            println!("  {}", style(&df.uuid).dim());
        }
    }

    // update the uuids
    if let Some(android_manifest) = android_manifest {
        api.associate_android_proguard_mappings(
            &org, &project, &android_manifest, all_checksums)?;

    // if values are given associate
    } else if let Some(app_id) = matches.value_of("app_id") {
        api.associate_dsyms(&org, &project, &AssociateDsyms {
            platform: matches.value_of("platform").unwrap_or("android").to_string(),
            checksums: all_checksums,
            name: app_id.to_string(),
            app_id: app_id.to_string(),
            version: matches.value_of("version").unwrap().to_string(),
            build: matches.value_of("version_code").map(|x| x.to_string()),
        })?;
    }

    // If wanted trigger reprocessing
    if !matches.is_present("no_reprocessing") &&
       !matches.is_present("no_upload") {
        if !api.trigger_reprocessing(&org, &project)? {
            println!("{} Server does not support reprocessing. Not triggering.",
                     style(">").dim());
        }
    } else {
        println!("{} skipped reprocessing", style(">").dim());
    }

    Ok(())
}
