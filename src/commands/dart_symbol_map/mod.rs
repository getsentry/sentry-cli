use anyhow::Result;
use clap::{ArgMatches, Command};

use crate::utils::args::ArgExt as _;

pub mod upload;

const GROUP_ABOUT: &str = "Manage Dart/Flutter symbol maps for Sentry.";

pub(super) fn make_command(mut command: Command) -> Command {
    command = command
        .about(GROUP_ABOUT)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(false);

    command = command.subcommand(upload::make_command(Command::new("upload")));
    command
}

pub(super) fn execute(matches: &ArgMatches) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("upload") {
        return upload::execute(sub_matches);
    }
    unreachable!();
}
