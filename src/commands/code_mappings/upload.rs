use std::fs;

use anyhow::{bail, Context as _, Result};
use clap::{Arg, ArgMatches, Command};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::utils::vcs;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodeMapping {
    stack_root: String,
    source_root: String,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload code mappings for a project from a JSON file.")
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
                .help("The default branch name. Defaults to the git remote HEAD or 'main'."),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    #[expect(clippy::unwrap_used, reason = "path is a required argument")]
    let path = matches.get_one::<String>("path").unwrap();
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

    // Resolve repo name and default branch
    let explicit_repo = matches.get_one::<String>("repo");
    let explicit_branch = matches.get_one::<String>("default_branch");

    let (repo_name, default_branch) = match (explicit_repo, explicit_branch) {
        (Some(r), Some(b)) => (r.to_owned(), b.to_owned()),
        _ => {
            let git_repo = git2::Repository::open_from_env().map_err(|e| {
                anyhow::anyhow!(
                    "Could not open git repository: {e}. \
                     Use --repo and --default-branch to specify manually."
                )
            })?;
            // Prefer explicit config (SENTRY_VCS_REMOTE / ini), then inspect
            // the repo for the best remote (upstream > origin > first).
            let config = Config::current();
            let configured_remote = config.get_cached_vcs_remote();
            let remote_name = if vcs::git_repo_remote_url(&git_repo, &configured_remote).is_ok() {
                debug!("Using configured VCS remote: {configured_remote}");
                configured_remote
            } else if let Some(best) = vcs::find_best_remote(&git_repo)? {
                debug!("Configured remote '{configured_remote}' not found, using: {best}");
                best
            } else {
                bail!(
                    "No remotes found in the git repository. \
                         Use --repo and --default-branch to specify manually."
                );
            };

            let repo_name = match explicit_repo {
                Some(r) => r.to_owned(),
                None => {
                    let remote_url = vcs::git_repo_remote_url(&git_repo, &remote_name)?;
                    debug!("Found remote '{remote_name}': {remote_url}");
                    let inferred = vcs::get_repo_from_remote(&remote_url);
                    if inferred.is_empty() {
                        bail!("Could not parse repository name from remote URL: {remote_url}");
                    }
                    println!("Inferred repository: {inferred}");
                    inferred
                }
            };

            let default_branch = match explicit_branch {
                Some(b) => b.to_owned(),
                None => {
                    let inferred =
                        vcs::git_repo_base_ref(&git_repo, &remote_name).unwrap_or_else(|e| {
                            debug!("Could not infer default branch, falling back to 'main': {e}");
                            "main".to_owned()
                        });
                    println!("Inferred default branch: {inferred}");
                    inferred
                }
            };

            (repo_name, default_branch)
        }
    };

    println!("Found {} code mapping(s) in {path}", mappings.len());
    println!("Repository: {repo_name}");
    println!("Default branch: {default_branch}");

    Ok(())
}
