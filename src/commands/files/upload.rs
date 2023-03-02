use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, format_err, Result};
use clap::{Arg, ArgMatches, Command, ArgAction};
use log::warn;
use symbolic::debuginfo::sourcebundle::SourceFileType;

use crate::api::{Api, ProgressBarMode};
use crate::config::Config;
use crate::utils::args::validate_distribution;
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{ReleaseFile, ReleaseFileUpload, UploadContext};
use crate::utils::fs::{decompress_gzip_content, is_gzip_compressed, path_as_url};

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload files for a release.")
        // Backward compatibility with `releases files <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path to the file or directory to upload."),
        )
        .arg(
            Arg::new("name")
                .value_name("NAME")
                .help("The name of the file on the server."),
        )
        .arg(
            Arg::new("dist")
                .long("dist")
                .short('d')
                .value_name("DISTRIBUTION")
                .value_parser(validate_distribution)
                .help("Optional distribution identifier for this file."),
        )
        .arg(
            Arg::new("decompress")
                .long("decompress")
                .help("Enable files gzip decompression prior to upload."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .help("Wait for the server to fully process uploaded files."),
        )
        .arg(
            Arg::new("file-headers")
                .long("file-header")
                .short('H')
                .value_name("KEY VALUE")
                .action(ArgAction::Append)
                .help("Store a header with this file."),
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
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let release = config.get_release_with_legacy_fallback(matches)?;
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let api = Api::current();

    let dist = matches.get_one::<String>("dist").map(String::as_str);
    let mut headers = vec![];
    if let Some(header_list) = matches.get_many::<String>("file-headers") {
        for header in header_list {
            if !header.contains(':') {
                bail!("Invalid header. Needs to be in key:value format");
            }
            let (key, value) = header.split_once(':').unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    };

    let context = &UploadContext {
        org: &org,
        project: project.as_deref(),
        release: &release,
        dist,
        wait: matches.contains_id("wait"),
        ..Default::default()
    };

    let path = Path::new(matches.get_one::<String>("path").unwrap());
    // Batch files upload
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
            .decompress(matches.contains_id("decompress"))
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

                (
                    url.to_string(),
                    ReleaseFile {
                        url,
                        path: source.path.clone(),
                        contents: source.contents.clone(),
                        ty: SourceFileType::Source,
                        headers: headers.clone(),
                        messages: vec![],
                        already_uploaded: false,
                    },
                )
            })
            .collect();

        ReleaseFileUpload::new(context).files(&files).upload()
    }
    // Single file upload
    else {
        let name = match matches.get_one::<String>("name") {
            Some(name) => name,
            None => Path::new(path)
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| format_err!("No filename provided."))?,
        };

        let mut f = fs::File::open(path)?;
        let mut contents = Vec::new();
        f.read_to_end(&mut contents)?;

        if matches.contains_id("decompress") && is_gzip_compressed(&contents) {
            contents = decompress_gzip_content(&contents).unwrap_or_else(|_| {
                warn!("Could not decompress: {}", name);
                contents
            });
        }

        if let Some(artifact) = api.upload_release_file(
            context,
            &contents,
            name,
            Some(&headers[..]),
            ProgressBarMode::Request,
        )? {
            println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
        } else {
            bail!("File already present!");
        }
        Ok(())
    }
}
