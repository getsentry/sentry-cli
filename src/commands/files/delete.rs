use std::collections::HashSet;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::api::Api;
use crate::config::Config;

pub fn make_command(command: Command) -> Command {
    command
        .about("Delete a release file.")
        // Backward compatibility with `releases files <VERSION>` commands.
        .arg(Arg::new("version").long("version").hide(true))
        .arg(
            Arg::new("names")
                .value_name("NAMES")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Filenames to delete."),
        )
        .arg(
            Arg::new("all")
                .short('A')
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Delete all files."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let release = config.get_release_with_legacy_fallback(matches)?;
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let api = Api::current();
    let authenticated_api = api.authenticated()?;

    if matches.get_flag("all") {
        authenticated_api.delete_release_files(&org, project.as_deref(), &release)?;
        println!("All files deleted.");
        return Ok(());
    }

    let files: HashSet<String> = match matches.get_many::<String>("names") {
        Some(paths) => paths.map(|x| x.into()).collect(),
        None => HashSet::new(),
    };
    for file in authenticated_api.list_release_files(&org, project.as_deref(), &release)? {
        if !files.contains(&file.name) {
            continue;
        }
        if authenticated_api.delete_release_file(&org, project.as_deref(), &release, &file.id)? {
            println!("D {}", file.name);
        }
    }
    Ok(())
}
