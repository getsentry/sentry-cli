mod list;

use self::list::ListLogsArgs;
use super::derive_parser::{SentryCLI, SentryCLICommand};
use anyhow::Result;
use clap::ArgMatches;
use clap::{Args, Command, Parser as _, Subcommand};

const BETA_WARNING: &str = "[BETA] The \"logs\" command is in beta. The command is subject \
    to breaking changes, including removal, in any Sentry CLI release.";

const LIST_ABOUT: &str = "List logs from your organization";

#[derive(Args)]
pub(super) struct LogsArgs {
    #[command(subcommand)]
    subcommand: LogsSubcommand,
}

#[derive(Subcommand)]
#[command(about = "[BETA] Manage logs in Sentry")]
#[command(long_about = format!(
    "Manage and query logs in Sentry. \
    This command provides access to log entries.\n\n\
    {BETA_WARNING}")
)]
enum LogsSubcommand {
    #[command(about = format!("[BETA] {LIST_ABOUT}"))]
    #[command(long_about = format!("{LIST_ABOUT}. \
    Query and filter log entries from your Sentry projects. \
    Supports filtering by log level and custom queries.\n\n\
    {BETA_WARNING}")
)]
    List(ListLogsArgs),
}

pub(super) fn make_command(command: Command) -> Command {
    LogsSubcommand::augment_subcommands(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::Logs(LogsArgs { subcommand }) = SentryCLI::parse().command;
    eprintln!("{BETA_WARNING}");

    match subcommand {
        LogsSubcommand::List(args) => list::execute(args),
    }
}
