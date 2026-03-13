use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::Table;

pub fn make_command(command: Command) -> Command {
    command.about("List project environments.").arg(
        Arg::new("show_hidden")
            .long("show-hidden")
            .action(clap::ArgAction::SetTrue)
            .help("Show hidden environments."),
    )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let show_hidden = matches.get_flag("show_hidden");
    let mut table = Table::new();
    table.title_row().add("Name").add("Hidden");
    let mut hidden_count = 0usize;

    for env in api
        .authenticated()?
        .list_project_environments(&config.get_org(matches)?, &config.get_project(matches)?)?
    {
        if env.is_hidden && !show_hidden {
            hidden_count += 1;
            continue;
        }
        table
            .add_row()
            .add(&env.name)
            .add(if env.is_hidden { "yes" } else { "no" });
    }

    if table.is_empty() {
        if hidden_count > 0 {
            println!(
                "No visible environments found ({} hidden, use --show-hidden to see them)",
                hidden_count
            );
        } else {
            println!("No environments found");
        }
    } else {
        table.print();
    }

    Ok(())
}
