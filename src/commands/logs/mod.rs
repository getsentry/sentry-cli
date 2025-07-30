mod list;

use self::list::ListLogsArgs;
use super::derive_parser::{SentryCLI, SentryCLICommand};
use anyhow::Result;
use clap::ArgMatches;
use clap::{Args, Command, Parser as _, Subcommand};

const LIST_ABOUT: &str = "List logs from your organization";

#[derive(Args)]
pub(super) struct LogsArgs {
    #[command(subcommand)]
    subcommand: LogsSubcommand,
}

#[derive(Subcommand)]
#[command(about = "Manage logs in Sentry")]
#[command(long_about = "Manage and query logs in Sentry. \
    This command provides access to log entries.")]
enum LogsSubcommand {
    #[command(about = LIST_ABOUT)]
    #[command(long_about = format!("{LIST_ABOUT}. \
    Query and filter log entries from your Sentry projects. \
    Supports filtering by time period, log level, and custom queries."))]
    List(ListLogsArgs),
}

pub(super) fn make_command(command: Command) -> Command {
    LogsSubcommand::augment_subcommands(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::Logs(LogsArgs { subcommand }) = SentryCLI::parse().command else {
        unreachable!("expected logs subcommand");
    };

    match subcommand {
        LogsSubcommand::List(args) => list::execute(args),
    }
}
