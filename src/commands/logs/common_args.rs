use clap::Args;

/// Common arguments for all logs subcommands.
#[derive(Args)]
pub(super) struct CommonLogsArgs {
    #[arg(short = 'o', long = "org")]
    #[arg(help = "The organization ID or slug.")]
    pub(super) org: Option<String>,

    #[arg(short = 'p', long = "project")]
    #[arg(help = "The project ID or slug.")]
    pub(super) project: Option<String>,
}
