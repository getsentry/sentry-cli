use anyhow::{Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use symbolic::common::ByteView;

use crate::utils::proguard::ProguardMapping;

pub fn make_command(command: Command) -> Command {
    command
        .about("Compute the UUID for a ProGuard mapping file.")
        .long_about(
            "Compute the UUID for a ProGuard mapping file.\n\n\
            This command computes and prints to stdout the UUID of the ProGuard \
            mapping at the specified path. This is the UUID that will be set by \
            the `proguard upload` command. The UUID is deterministicly computed \
            based on the file contents.",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .help("The path to the mapping file.")
                .required(true),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = matches
        .get_one::<String>("path")
        .expect("required argument");

    let byteview = ByteView::open(path)
        .with_context(|| format!("failed to open proguard mapping '{path}'"))?;
    let mapping = ProguardMapping::from(byteview);

    println!("{}", mapping.uuid());
    Ok(())
}
