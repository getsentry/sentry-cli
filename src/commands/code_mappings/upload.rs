use std::fs;

use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodeMapping {
    stack_root: String,
    source_root: String,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload code mappings for a project from a JSON file. Each mapping pairs a stack trace root (e.g. com/example/module) with the corresponding source path in your repository (e.g. modules/module/src/main/java/com/example/module).")
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("Path to a JSON file containing code mappings."),
        )
        .arg(
            Arg::new("repo")
                .long("repo")
                .value_name("REPO")
                .help("The repository name (e.g. owner/repo). Defaults to the git remote."),
        )
        .arg(
            Arg::new("default_branch")
                .long("default-branch")
                .value_name("BRANCH")
                .default_value("main")
                .help("The default branch name."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = matches
        .get_one::<String>("path")
        .expect("path is a required argument");
    let data = fs::read(path).with_context(|| format!("Failed to read mappings file '{path}'"))?;

    let mappings: Vec<CodeMapping> =
        serde_json::from_slice(&data).context("Failed to parse mappings JSON")?;

    if mappings.is_empty() {
        bail!("Mappings file contains an empty array. Nothing to upload.");
    }

    for (i, mapping) in mappings.iter().enumerate() {
        if mapping.stack_root.is_empty() {
            bail!("Mapping at index {i} has an empty stackRoot.");
        }
        if mapping.source_root.is_empty() {
            bail!("Mapping at index {i} has an empty sourceRoot.");
        }
    }

    println!("Found {} code mapping(s) in {path}", mappings.len());

    Ok(())
}
