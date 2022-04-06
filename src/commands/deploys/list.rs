use anyhow::Result;
use chrono::Utc;
use clap::{Arg, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;
use crate::utils::formatting::{HumanDuration, Table};

pub fn make_command(command: Command) -> Command {
    command
        .about("List all deployments of a release.")
        // Backward compatibility with `releases deploys <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = config.get_release_with_legacy_fallback(matches)?;
    let mut table = Table::new();
    table
        .title_row()
        .add("Environment")
        .add("Name")
        .add("Finished");

    for deploy in api.list_deploys(&config.get_org(matches)?, &version)? {
        table
            .add_row()
            .add(&deploy.env)
            .add(deploy.name())
            .add(HumanDuration(
                Utc::now().signed_duration_since(deploy.finished.unwrap()),
            ));
    }

    if table.is_empty() {
        println!("No deploys found");
    } else {
        table.print();
    }

    Ok(())
}
