use super::value_parsers::kv_parser;
use crate::utils::auth_token::AuthToken;
use clap::{command, value_parser, ArgAction::SetTrue, Parser, Subcommand};

#[derive(Parser)]
pub(crate) struct SentryCLI {
    #[command(subcommand)]
    pub(crate) command: SentryCLICommand,

    #[arg(global=true, long="header", value_name="KEY:VALUE", value_parser=kv_parser, help="Custom headers that should be attached to all requests{n}in key:value format")]
    pub(crate) headers: Vec<(String, String)>,

    #[arg(global=true, long, value_parser=value_parser!(AuthToken), help="Use the given Sentry auth token")]
    pub(crate) auth_token: Option<AuthToken>,

    #[arg(global=true, ignore_case=true, long, value_parser=["trace", "debug", "info", "warn", "error"], help="Set the log output verbosity")]
    pub(crate) log_level: Option<String>,

    #[arg(global=true, action=SetTrue, visible_alias="silent", long, help="Do not print any output while preserving correct exit code. This flag is currently implemented only for selected subcommands")]
    pub(crate) quiet: bool,

    #[arg(global=true, action=SetTrue, long, hide=true, help="Always return 0 exit code")]
    pub(crate) allow_failure: bool,
}

#[derive(Subcommand)]
pub(crate) enum SentryCLICommand {}
