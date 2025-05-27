use anyhow::Result;
use clap::ArgAction;
use clap::{Arg, ArgMatches, Command};

use crate::utils::args::ArgExt;

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload mobile app files to a project.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("paths")
                .value_name("PATH")
                .help("The path to the mobile app files to upload. Supported files include Apk, Aab or XCArchive.")
                .num_args(1..)
                .action(ArgAction::Append),
        )
}

#[allow(clippy::unnecessary_wraps)]
pub fn execute(_matches: &ArgMatches) -> Result<()> {
    println!("Uploading mobile app files to a project is not yet implemented.");

    Ok(())
}
