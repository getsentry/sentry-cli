use anyhow::Result;
use clap::{ArgMatches, Args, Command, Parser as _, Subcommand};

pub mod upload;

const GROUP_ABOUT: &str = "Manage Dart/Flutter symbol maps for Sentry.";
const UPLOAD_ABOUT: &str =
    "Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.";
const UPLOAD_LONG_ABOUT: &str =
    "Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.{n}{n}Examples:{n}  sentry-cli dart-symbol-map upload --org my-org --project my-proj path/to/dartsymbolmap.json path/to/debug/file{n}{n}The mapping must be a JSON array of strings with an even number of entries (pairs).{n}The debug file must contain exactly one Debug ID.";

#[derive(Args)]
pub(super) struct DartSymbolMapArgs {
    #[command(subcommand)]
    pub(super) subcommand: DartSymbolMapSubcommand,
}

#[derive(Subcommand)]
#[command(about = GROUP_ABOUT)]
pub(super) enum DartSymbolMapSubcommand {
    #[command(about = UPLOAD_ABOUT)]
    #[command(long_about = UPLOAD_LONG_ABOUT)]
    Upload(upload::DartSymbolMapUploadArgs),
}

pub(super) fn make_command(command: Command) -> Command {
    DartSymbolMapSubcommand::augment_subcommands(
        command
            .about(GROUP_ABOUT)
            .subcommand_required(true)
            .arg_required_else_help(true),
    )
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let subcommand = match crate::commands::derive_parser::SentryCLI::parse().command {
        crate::commands::derive_parser::SentryCLICommand::DartSymbolMap(DartSymbolMapArgs {
            subcommand,
        }) => subcommand,
        _ => unreachable!("expected dart-symbol-map subcommand"),
    };

    match subcommand {
        DartSymbolMapSubcommand::Upload(args) => upload::execute(args),
    }
}
