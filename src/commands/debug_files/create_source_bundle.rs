use std::fs;
use std::path::{Path};
use anyhow::{bail, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use symbolic::debuginfo::sourcebundle::{SourceFileType};
use crate::api::Api;
use crate::config::Config;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{FileUpload, SourceFile, UploadContext};
use crate::utils::fs::{path_as_url};

pub fn make_command(command: Command) -> Command {
    command
        .about("Create a source bundle for the given source files")
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path to the directory containing source files to bundle."),
        )
        // TODO org and project shouldn't be needed explicitly here, ask kamil
        .arg(
            Arg::new("org")
                .long("org")
                .value_name("ORGANIZATION")
                .help("TODO Get rid of this"),
        )
        .arg(
            Arg::new("project")
                .long("project")
                .value_name("PROJECT")
                .help("TODO Get rid of this too"),
        )
        .arg(
            Arg::new("url_prefix")
                .short('u')
                .long("url-prefix")
                .value_name("PREFIX")
                .help("The URL prefix to prepend to all filenames."),
        )
        .arg(
            Arg::new("url_suffix")
                .long("url-suffix")
                .value_name("SUFFIX")
                .help("The URL suffix to append to all filenames."),
        )
        .arg(
            Arg::new("ignore")
                .long("ignore")
                .short('i')
                .value_name("IGNORE")
                .action(ArgAction::Append)
                .help("Ignores all files and folders matching the given glob"),
        )
        .arg(
            Arg::new("ignore_file")
                .long("ignore-file")
                .short('I')
                .value_name("IGNORE_FILE")
                .help(
                    "Ignore all files and folders specified in the given \
                    ignore file, e.g. .gitignore.",
                ),
        )
        .arg(
            Arg::new("extensions")
                .long("ext")
                .short('x')
                .value_name("EXT")
                .action(ArgAction::Append)
                .help(
                    "Set the file extensions that are considered for upload. \
                    This overrides the default extensions. To add an extension, all default \
                    extensions must be repeated. Specify once per extension.",
                ),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("PATH")
                .help(
                    "The path to the output folder.  If not provided the \
                     file is placed TODO.",
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    // let release = config.get_release_with_legacy_fallback(matches)?;
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    // let dist = matches.get_one::<String>("dist").map(String::as_str);
    let api = Api::current();
    let chunk_upload_options = api.get_chunk_upload_options(&org)?;
    let context = &UploadContext {
        org: &org,
        project: project.as_deref(),
        release: None, // release: Some(&release),
        dist: None, // dist,
        note: None,
        wait: true, //matches.get_flag("wait"),
        dedupe: false,
        chunk_upload_options: chunk_upload_options.as_ref(),
    };
    let path = Path::new(matches.get_one::<String>("path").unwrap());
    let fallback_dir = Path::new("/Users/adinauer/dev/debug/");
    let output_path = matches.get_one::<String>("output").map(Path::new);
    let out = output_path.unwrap_or(fallback_dir).join(Path::new("source_bundle.zip"));

    if path.is_dir() {
        let ignore_file = matches
            .get_one::<String>("ignore_file")
            .map(String::as_str)
            .unwrap_or_default();
        let ignores = matches
            .get_many::<String>("ignore")
            .map(|ignores| ignores.map(|i| format!("!{i}")).collect())
            .unwrap_or_else(Vec::new);
        let extensions = matches
            .get_many::<String>("extensions")
            .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
            .unwrap_or_else(Vec::new);

        let sources = ReleaseFileSearch::new(path.to_path_buf())
            .ignore_file(ignore_file)
            .ignores(ignores)
            .extensions(extensions)
            // .decompress(matches.get_flag("decompress"))
            .collect_files()?;

        let url_suffix = matches
            .get_one::<String>("url_suffix")
            .map(String::as_str)
            .unwrap_or_default();
        let mut url_prefix = matches
            .get_one::<String>("url_prefix")
            .map(String::as_str)
            .unwrap_or("~");
        // remove a single slash from the end.  so ~/ becomes ~ and app:/// becomes app://
        if url_prefix.ends_with('/') {
            url_prefix = &url_prefix[..url_prefix.len() - 1];
        }
        let files = sources
            .iter()
            .map(|source| {
                let local_path = source.path.strip_prefix(&source.base_path).unwrap();
                let url = format!("{}/{}{}", url_prefix, path_as_url(local_path), url_suffix);

                println!("file {}", source.path.to_string_lossy());
                println!("basepath {}", source.base_path.to_string_lossy());
                println!("url {}", url);

                (
                    url.to_string(),
                    SourceFile {
                        url,
                        path: source.path.clone(),
                        contents: source.contents.clone(),
                        ty: SourceFileType::Source,
                        // headers: headers.clone(),
                        headers: vec![],
                        messages: vec![],
                        already_uploaded: false,
                    },
                )
            })
            .collect();

        let rename_result = match FileUpload::new(context).files(&files).build_source_bundle() {
            Ok(tempfile) => fs::rename(tempfile.path(), &out),
            Err(e) => bail!("Unable to create source bundle... {}", e),
        };
        match rename_result {
            Ok(()) => println!("created {}", out.to_string_lossy()),
            Err(e) => bail!("err asdf12 {}", e)
        }
        Ok(())
    } else {
        // TODO handle file
        Ok(())
    }
}
