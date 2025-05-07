
use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::utils::args::validate_distribution;

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
                .action(ArgAction::SetTrue)
                .help("Enable files gzip decompression prior to upload."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .conflicts_with("wait_for")
                .help("Wait for the server to fully process uploaded files."),
        )
        .arg(
            Arg::new("wait_for")
                .long("wait-for")
                .value_name("SECS")
                .value_parser(clap::value_parser!(u64))
                .conflicts_with("wait")
                .help(
                    "Wait for the server to fully process uploaded files, \
                     but at most for the given number of seconds.",
                ),
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

pub fn execute(_matches: &ArgMatches) -> Result<()> {
    unimplemented!("CI should not pass with this change.");
}
