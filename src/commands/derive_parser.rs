use crate::utils::auth_token::AuthToken;
use crate::utils::value_parsers::{auth_token_parser, kv_parser};
use clap::{command, ArgAction::SetTrue, Parser, Subcommand};

use super::send_metric::SendMetricArgs;

#[derive(Parser)]
pub(super) struct SentryCLI {
    #[command(subcommand)]
    pub(super) command: SentryCLICommand,

    #[arg(global=true, long="header", value_name="KEY:VALUE", value_parser=kv_parser)]
    #[arg(help = "Custom headers that should be attached to all requests{n}in key:value format")]
    pub(super) headers: Vec<(String, String)>,

    #[arg(global=true, long, value_parser=auth_token_parser)]
    #[arg(help = "Use the given Sentry auth token")]
    pub(super) auth_token: Option<AuthToken>,

    #[arg(global=true, ignore_case=true, value_parser=["trace", "debug", "info", "warn", "error"])]
    #[arg(long, help = "Set the log output verbosity")]
    pub(super) log_level: Option<String>,

    #[arg(global=true, action=SetTrue, visible_alias="silent", long)]
    #[arg(help = "Do not print any output while preserving correct exit code. \
        This flag is currently implemented only for selected subcommands")]
    pub(super) quiet: bool,

    #[arg(global=true, action=SetTrue, long, hide=true, help="Always return 0 exit code")]
    pub(super) allow_failure: bool,
}

#[derive(Subcommand)]
pub(super) enum SentryCLICommand {
    SendMetric(SendMetricArgs),
}
