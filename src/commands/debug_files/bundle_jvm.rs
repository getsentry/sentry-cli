use crate::api::Api;
use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::ArgExt;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{FileUpload, SourceFile, UploadContext};
use crate::utils::fs::path_as_url;
use anyhow::{bail, Context, Result};
use clap::{Arg, ArgMatches, Command};
use sentry::types::DebugId;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use symbolic::debuginfo::sourcebundle::SourceFileType;

pub fn make_command(command: Command) -> Command {
    command
        .hide(true) // experimental for now
        .about(
            "Create a source bundle for the given JVM based source files (e.g. Java, Kotlin, ...)",
        )
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new())
                .help("The directory containing source files to bundle."),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .value_name("PATH")
                .required(true)
                .value_parser(clap::builder::PathBufValueParser::new())
                .help("The path to the output folder."),
        )
        .arg(
            Arg::new("debug_id")
                .long("debug-id")
                .value_name("UUID")
                .required(true)
                .value_parser(DebugId::from_str)
                .help("Debug ID (UUID) to use for the source bundle."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let api = Api::current();
    let chunk_upload_options = api.authenticated()?.get_chunk_upload_options(&org)?;

    let context = &UploadContext {
        org: &org,
        project: project.as_deref(),
        release: None,
        dist: None,
        note: None,
        wait: true,
        max_wait: DEFAULT_MAX_WAIT,
        dedupe: false,
        chunk_upload_options: chunk_upload_options.as_ref(),
    };
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let output_path = matches.get_one::<PathBuf>("output").unwrap();
    let debug_id = matches.get_one::<DebugId>("debug_id").unwrap();
    let out = output_path.join(format!("{debug_id}.zip"));

    if !path.exists() {
        bail!("Given path does not exist: {}", path.display())
    }

    if !path.is_dir() {
        bail!("Given path is not a directory: {}", path.display())
    }

    if !output_path.exists() {
        fs::create_dir_all(output_path).context(format!(
            "Failed to create output directory {}",
            output_path.display()
        ))?;
    }

    let sources = ReleaseFileSearch::new(path.to_path_buf()).collect_files()?;
    let files = sources
        .iter()
        .map(|source| {
            let local_path = source.path.strip_prefix(&source.base_path).unwrap();
            let local_path_jvm_ext = local_path.with_extension("jvm");
            let url = format!("~/{}", path_as_url(&local_path_jvm_ext));
            (
                url.to_string(),
                SourceFile {
                    url,
                    path: source.path.clone(),
                    contents: source.contents.clone(),
                    ty: SourceFileType::Source,
                    headers: BTreeMap::new(),
                    messages: vec![],
                    already_uploaded: false,
                },
            )
        })
        .collect();

    let tempfile = FileUpload::new(context)
        .files(&files)
        .build_jvm_bundle(Some(*debug_id))
        .context("Unable to create source bundle")?;

    fs::copy(tempfile.path(), &out).context("Unable to write source bundle")?;
    println!("Created {}", out.display());

    Ok(())
}
