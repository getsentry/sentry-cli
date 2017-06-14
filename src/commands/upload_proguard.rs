//! Implements a command for uploading proguard mapping files.
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use prelude::*;
use utils::{ArgExt, TempFile, copy_with_progress, make_byte_progress_bar,
            get_sha1_checksum, AndroidManifest};
use config::Config;
use api::{Api, AssociateDsyms};

use clap::{App, Arg, ArgMatches};
use uuid::Uuid;
use zip;
use proguard::MappingView;
use console::style;

#[derive(Debug)]
struct MappingRef {
    pub path: PathBuf,
    pub size: u64,
    pub uuid: Uuid,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads proguard mapping files to a project")
        .org_project_args()
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .help("The path to the mapping files")
            .multiple(true)
            .number_of_values(1)
            .index(1))
        .arg(Arg::with_name("version")
             .long("version")
             .value_name("VERSION")
             .requires("app_id")
             .help("Optionally associate the mapping files with a humand \
                    readable version.  This helps you understand which \
                    proguard files go with which version of your app."))
        .arg(Arg::with_name("version_code")
             .long("version-code")
             .value_name("VERSION_CODE")
             .requires("app_id")
             .requires("version")
             .help("Optionally associate the mapping files with a version \
                    code.  This helps you understand which proguard files \
                    go with which version of your app."))
        .arg(Arg::with_name("app_id")
             .long("app-id")
             .value_name("APP_ID")
             .requires("version")
             .help("Optionally associate the mapping files with an application \
                    ID.  If you have multiple apps in one sentry project you can \
                    then easlier tell them apart."))
        .arg(Arg::with_name("platform")
             .long("platform")
             .value_name("PLATFORM")
             .requires("app_id")
             .help("Optionally defines the platform for the app association. \
                    This defaults to 'android'."))
        .arg(Arg::with_name("no_reprocessing")
             .long("no-reprocessing")
             .help("Does not trigger reprocessing after upload"))
        .arg(Arg::with_name("no_upload")
             .long("no-upload")
             .help("Disables the actual upload.  This runs all steps for the \
                    processing but does not trigger the upload (this also \
                    automatically disables reprocessing.  This is useful if you \
                    just want to verify the mapping files and update manifests."))
        .arg(Arg::with_name("android_manifest")
             .long("android-manifest")
             .value_name("PATH")
             .conflicts_with("app_id")
             .help("If provided the path to a processed android manifest \
                    needs to be supplied.  In that case version and version \
                    code as well as the package are read from there. \
                    Additionally if --update-manifest is provided the \
                    manifest is updated with the proguard UUID."))
        .arg(Arg::with_name("update_manifest")
             .long("update-manifest")
             .help("If provided the android manifest is updated with the \
                    build UUID.")
             .requires("android_manifest"))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let (org, project) = config.get_org_and_project(matches)?;
    let api = Api::new(config);

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

    // since the mappings are quite small we don't bother doing a second http
    // request to figure out if any of the checksums are missing.  We just ship
    // them all up.
    for path in &paths {
        let md = fs::metadata(path)?;
        let mapping = MappingView::from_path(path)?;
        if !mapping.has_line_info() {
            println_stderr!("warning: proguard mapping '{}' was ignored because it \
                             does not contain any line information.", path);
        } else {
            let mut f = fs::File::open(path)?;
            all_checksums.push(get_sha1_checksum(&mut f)?);
            mappings.push(MappingRef {
                path: PathBuf::from(path),
                size: md.len(),
                uuid: mapping.uuid(),
            });
        }
    }

    if mappings.is_empty() {
        fail!("found no mapping files to upload");
    }

    println!("{} compressing mappings", style("[1/2]").dim());
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

    if !matches.is_present("no_upload") {
        println!("{} uploading mappings", style("[2/2]").dim());
        let rv = api.upload_dsyms(&org, &project, tf.path())?;
    } else {
        println!("Skipping upload.");
    }

    println!("Uploaded a total of {} new mapping files",
             style(rv.len()).yellow());

    if rv.len() > 0 {
        println!("Newly uploaded debug symbols:");
        for df in rv {
            println!("  {}", style(&df.uuid).dim());
        }
    }

    // update the uuids
    if let Some(mut android_manifest) = android_manifest {
        let uuids: Vec<_> = mappings.iter().map(|x| x.uuid).collect();
        if matches.is_present("update_manifest") {
            android_manifest.set_proguard_uuids(&uuids);
            android_manifest.save()?;
        }
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
            println!("Server does not support reprocessing. Not triggering.");
        }
    } else {
        println!("Skipped reprocessing.");
    }

    Ok(())
}
