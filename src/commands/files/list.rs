use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use indicatif::HumanBytes;

use crate::{api::Api, config::Config, utils::formatting::Table};

pub fn make_command(command: Command) -> Command {
    command
        .about("List all release files.")
        // Backward compatibility with `releases files <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let release = config.get_release_with_legacy_fallback(matches)?;
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let api = Api::current();

    let mut table = Table::new();
    table
        .title_row()
        .add("Name")
        .add("Distribution")
        .add("Source Map")
        .add("Size");

    for artifact in api.list_release_files(&org, project.as_deref(), &release)? {
        let row = table.add_row();
        row.add(&artifact.name);
        if let Some(ref dist) = artifact.dist {
            row.add(dist);
        } else {
            row.add("");
        }
        if let Some(sm_ref) = artifact.get_sourcemap_reference() {
            row.add(sm_ref);
        } else {
            row.add("");
        }
        row.add(HumanBytes(artifact.size));
    }

    table.print();

    Ok(())
}
