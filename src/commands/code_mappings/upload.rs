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
                .help("The default branch name. Defaults to the git remote HEAD or 'main'."),
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

    // Resolve repo name and default branch, falling back to git inference.
    let explicit_repo = matches.get_one::<String>("repo");
    let explicit_branch = matches.get_one::<String>("default_branch");

    let git_repo = (explicit_repo.is_none() || explicit_branch.is_none())
        .then(|| git2::Repository::open_from_env().ok())
        .flatten();
    let remote_name = git_repo.as_ref().and_then(resolve_git_remote);

    let repo_name = if let Some(r) = explicit_repo {
        r.to_owned()
    } else {
        infer_repo_name(git_repo.as_ref(), remote_name.as_deref())?
    };

    let default_branch = if let Some(b) = explicit_branch {
        b.to_owned()
    } else {
        infer_default_branch(git_repo.as_ref(), remote_name.as_deref())
    };

    println!("Found {} code mapping(s) in {path}", mappings.len());
    println!("Repository: {repo_name}");
    println!("Default branch: {default_branch}");

    Ok(())
}

/// Finds the best git remote name. Prefers the configured VCS remote
/// (SENTRY_VCS_REMOTE / ini), then falls back to upstream > origin > first.
fn resolve_git_remote(repo: &git2::Repository) -> Option<String> {
    let config = Config::current();
    let configured_remote = config.get_cached_vcs_remote();
    if vcs::git_repo_remote_url(repo, &configured_remote).is_ok() {
        debug!("Using configured VCS remote: {configured_remote}");
        return Some(configured_remote);
    }
    match vcs::find_best_remote(repo) {
        Ok(Some(best)) => {
            debug!("Configured remote '{configured_remote}' not found, using: {best}");
            Some(best)
        }
        _ => None,
    }
}

/// Infers the repository name (e.g. "owner/repo") from the git remote URL.
fn infer_repo_name(
    git_repo: Option<&git2::Repository>,
    remote_name: Option<&str>,
) -> Result<String> {
    let git_repo = git_repo.ok_or_else(|| {
        anyhow::anyhow!("Could not open git repository. Use --repo to specify manually.")
    })?;
    let remote_name = remote_name.ok_or_else(|| {
        anyhow::anyhow!("No remotes found in the git repository. Use --repo to specify manually.")
    })?;
    let remote_url = vcs::git_repo_remote_url(git_repo, remote_name)?;
    debug!("Found remote '{remote_name}': {remote_url}");
    let inferred = vcs::get_repo_from_remote_preserve_case(&remote_url);
    if inferred.is_empty() {
        bail!("Could not parse repository name from remote URL: {remote_url}");
    }
    println!("Inferred repository: {inferred}");
    Ok(inferred)
}

/// Infers the default branch from the git remote HEAD, falling back to "main".
fn infer_default_branch(git_repo: Option<&git2::Repository>, remote_name: Option<&str>) -> String {
    let inferred = git_repo
        .zip(remote_name)
        .and_then(|(repo, name)| {
            vcs::git_repo_base_ref(repo, name)
                .map_err(|e| {
                    debug!("Could not infer default branch from remote: {e}");
                    e
                })
                .ok()
        })
        .unwrap_or_else(|| {
            debug!("No git repo or remote available, falling back to 'main'");
            "main".to_owned()
        });
    println!("Inferred default branch: {inferred}");
    inferred
}
