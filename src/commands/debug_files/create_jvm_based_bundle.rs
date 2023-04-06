use std::fs;
use std::path::{Path};
use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use sentry::types::DebugId;
use symbolic::debuginfo::sourcebundle::{SourceFileType};
use uuid::{Uuid};
use crate::api::Api;
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{FileUpload, SourceFile, UploadContext};
use crate::utils::fs::{path_as_url};

pub fn make_command(command: Command) -> Command {
    command
        .hide(true) // experimental for now
        .about("Create a source bundle for the given JVM based source files (e.g. Java, Kotlin, ...)")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The directory containing source files to bundle."),
        )
        .arg(
            Arg::new("output")
                // .short('o')
                .long("output")
                .value_name("PATH")
                .required(true)
                .help("The path to the output folder."),
        )
        .arg(
            Arg::new("debug_id")
                // .short('d')
                .long("debug-id")
                .value_name("UUID")
                .required(true)
                .help("Debug ID to use for the source bundle."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let api = Api::current();
    let chunk_upload_options = api.get_chunk_upload_options(&org)?;
    let context = &UploadContext {
        org: &org,
        project: project.as_deref(),
        release: None,
        dist: None,
        note: None,
        wait: true,
        dedupe: false,
        chunk_upload_options: chunk_upload_options.as_ref(),
    };
    let path = Path::new(matches.get_one::<String>("path").unwrap());
    let output_path = Path::new(matches.get_one::<String>("output").unwrap());
    let debug_id_arg = matches.get_one::<String>("debug_id").unwrap();
    let debug_id_uuid = Uuid::parse_str(debug_id_arg);
    if debug_id_uuid.is_err() {
        bail!("Given debug_id is invalid: {}", debug_id_arg)
    }
    let debug_id = DebugId::from_uuid(debug_id_uuid.unwrap());
    let mut debug_id_string = debug_id.to_string().to_owned();
    debug_id_string.push_str(".zip");
    let out = output_path.join(Path::new(&debug_id_string));

    if !path.exists() {
        bail!("Given path does not exist: {}", path.to_string_lossy())
    }

    if path.is_dir() {
        let sources = ReleaseFileSearch::new(path.to_path_buf()).collect_files()?;
        let files = sources
            .iter()
            .map(|source| {
                let local_path = source.path.strip_prefix(&source.base_path).unwrap();
                let local_path_jvm_ext = local_path.with_extension("jvm");
                let url = format!("{}/{}", "~", path_as_url(&local_path_jvm_ext));
                (
                    url.to_string(),
                    SourceFile {
                        url,
                        path: source.path.clone(),
                        contents: source.contents.clone(),
                        ty: SourceFileType::Source,
                        headers: vec![],
                        messages: vec![],
                        already_uploaded: false,
                    },
                )
            })
            .collect();

        let copy_result = match FileUpload::new(context).files(&files).build_jvm_based_bundle(Some(debug_id)) {
            Ok(tempfile) => fs::copy(tempfile.path(), &out),
            Err(e) => bail!("Unable to create source bundle: {}", e),
        };
        match copy_result {
            Ok(_) => println!("Created {}", out.to_string_lossy()),
            Err(e) => bail!("Unable to write source bundle: {}", e)
        }
        Ok(())
    } else {
        bail!("Given path is not a directory: {}", path.to_string_lossy())
    }
}
