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

    let (repo_name, default_branch) = resolve_repo_and_branch(matches)?;

    println!("Found {} code mapping(s) in {path}", mappings.len());
    println!("Repository: {repo_name}");
    println!("Default branch: {default_branch}");

    Ok(())
}

/// Resolves the repository name and default branch from CLI args and git inference.
fn resolve_repo_and_branch(matches: &ArgMatches) -> Result<(String, String)> {
    let explicit_repo = matches.get_one::<String>("repo");
    let explicit_branch = matches.get_one::<String>("default_branch");

    let git_repo = (explicit_repo.is_none() || explicit_branch.is_none())
        .then(|| git2::Repository::open_from_env().ok())
        .flatten();

    let (repo_name, remote_name) = if let Some(r) = explicit_repo {
        let remote = git_repo
            .as_ref()
            .and_then(|repo| find_remote_for_repo(repo, r));
        (r.to_owned(), remote)
    } else {
        let remote = git_repo.as_ref().and_then(resolve_git_remote);
        let name = infer_repo_name(git_repo.as_ref(), remote.as_deref())?;
        (name, remote)
    };

    let default_branch = if let Some(b) = explicit_branch {
        b.to_owned()
    } else {
        infer_default_branch(git_repo.as_ref(), remote_name.as_deref())
    };

    Ok((repo_name, default_branch))
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

/// Finds the remote whose URL matches the given repository name (e.g. "owner/repo").
fn find_remote_for_repo(repo: &git2::Repository, repo_name: &str) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    let found = remotes.iter().flatten().find(|name| {
        vcs::git_repo_remote_url(repo, name)
            .map(|url| vcs::get_repo_from_remote_preserve_case(&url) == repo_name)
            .unwrap_or(false)
    })?;
    debug!("Found remote '{found}' matching repo '{repo_name}'");
    Some(found.to_owned())
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
    Ok(inferred)
}

/// Infers the default branch from the git remote HEAD, falling back to "main".
fn infer_default_branch(git_repo: Option<&git2::Repository>, remote_name: Option<&str>) -> String {
    git_repo
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
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use ini::Ini;
    use serial_test::serial;
    use tempfile::tempdir;

    use crate::config::Config;

    fn init_git_repo_with_remotes(remotes: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().expect("temp dir");
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&dir)
            .env_remove("GIT_DIR")
            .output()
            .expect("git init");
        for (name, url) in remotes {
            std::process::Command::new("git")
                .args(["remote", "add", name, url])
                .current_dir(&dir)
                .output()
                .expect("git remote add");
        }
        dir
    }

    /// Creates a commit and sets up remote HEAD refs so branch inference works.
    fn setup_remote_head_refs(
        repo: &git2::Repository,
        dir: &std::path::Path,
        branches: &[(&str, &str)],
    ) {
        for (args, msg) in [
            (vec!["config", "--local", "user.name", "test"], "git config"),
            (
                vec!["config", "--local", "user.email", "test@test.com"],
                "git config",
            ),
            (vec!["commit", "--allow-empty", "-m", "init"], "git commit"),
        ] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(dir)
                .output()
                .expect(msg);
        }

        let head_commit = repo.head().unwrap().peel_to_commit().unwrap().id();
        for (remote, branch) in branches {
            repo.reference(
                &format!("refs/remotes/{remote}/{branch}"),
                head_commit,
                false,
                "test",
            )
            .unwrap();
            repo.reference_symbolic(
                &format!("refs/remotes/{remote}/HEAD"),
                &format!("refs/remotes/{remote}/{branch}"),
                false,
                "test",
            )
            .unwrap();
        }
    }

    /// Runs `resolve_repo_and_branch` with the given CLI args, pointing GIT_DIR
    /// at the temp repo. Returns the resolved (repo_name, default_branch).
    fn run_resolve(dir: &std::path::Path, args: &[&str]) -> Result<(String, String)> {
        // Point git2::Repository::open_from_env() at our temp repo.
        let old_git_dir = std::env::var("GIT_DIR").ok();
        std::env::set_var("GIT_DIR", dir.join(".git"));

        // Bind a default Config so resolve_git_remote can call Config::current().
        Config::from_file(PathBuf::from("/dev/null"), Ini::new()).bind_to_process();

        let cmd = make_command(Command::new("upload"));
        let matches = cmd.try_get_matches_from(args).expect("valid args");
        let result = resolve_repo_and_branch(&matches);

        // Restore GIT_DIR.
        match old_git_dir {
            Some(val) => std::env::set_var("GIT_DIR", val),
            None => std::env::remove_var("GIT_DIR"),
        }

        result
    }

    #[test]
    #[serial]
    fn find_remote_for_repo_matches_upstream() {
        let dir = init_git_repo_with_remotes(&[
            ("origin", "https://github.com/my-fork/MyRepo"),
            ("upstream", "https://github.com/MyOrg/MyRepo"),
        ]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        assert_eq!(
            find_remote_for_repo(&repo, "MyOrg/MyRepo"),
            Some("upstream".to_owned())
        );
    }

    #[test]
    #[serial]
    fn find_remote_for_repo_matches_origin() {
        let dir = init_git_repo_with_remotes(&[("origin", "https://github.com/MyOrg/MyRepo")]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        assert_eq!(
            find_remote_for_repo(&repo, "MyOrg/MyRepo"),
            Some("origin".to_owned())
        );
    }

    #[test]
    #[serial]
    fn find_remote_for_repo_no_match() {
        let dir =
            init_git_repo_with_remotes(&[("origin", "https://github.com/other-org/other-repo")]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        assert_eq!(find_remote_for_repo(&repo, "MyOrg/MyRepo"), None);
    }

    #[test]
    #[serial]
    fn find_remote_for_repo_preserves_case() {
        let dir = init_git_repo_with_remotes(&[("origin", "https://github.com/MyOrg/MyRepo")]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        assert_eq!(find_remote_for_repo(&repo, "myorg/myrepo"), None);
    }

    #[test]
    #[serial]
    fn resolve_no_repo_no_branch_infers_both() {
        let dir = init_git_repo_with_remotes(&[("origin", "https://github.com/MyOrg/MyRepo")]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        setup_remote_head_refs(&repo, dir.path(), &[("origin", "develop")]);

        let (repo_name, branch) = run_resolve(dir.path(), &["upload", "mappings.json"]).unwrap();
        assert_eq!(repo_name, "MyOrg/MyRepo");
        assert_eq!(branch, "develop");
    }

    #[test]
    #[serial]
    fn resolve_explicit_branch_no_repo_infers_repo() {
        let dir = init_git_repo_with_remotes(&[("origin", "https://github.com/MyOrg/MyRepo")]);

        let (repo_name, branch) = run_resolve(
            dir.path(),
            &["upload", "mappings.json", "--default-branch", "release"],
        )
        .unwrap();
        assert_eq!(repo_name, "MyOrg/MyRepo");
        assert_eq!(branch, "release");
    }

    #[test]
    #[serial]
    fn resolve_both_explicit_skips_git() {
        // Both --repo and --default-branch given: no git needed at all.
        let dir = tempdir().expect("temp dir");

        let (repo_name, branch) = run_resolve(
            dir.path(),
            &[
                "upload",
                "mappings.json",
                "--repo",
                "MyOrg/MyRepo",
                "--default-branch",
                "release",
            ],
        )
        .unwrap();
        assert_eq!(repo_name, "MyOrg/MyRepo");
        assert_eq!(branch, "release");
    }

    #[test]
    #[serial]
    fn resolve_explicit_repo_no_match_falls_back_to_main() {
        let dir =
            init_git_repo_with_remotes(&[("origin", "https://github.com/other-org/other-repo")]);

        let (repo_name, branch) = run_resolve(
            dir.path(),
            &["upload", "mappings.json", "--repo", "MyOrg/MyRepo"],
        )
        .unwrap();
        assert_eq!(repo_name, "MyOrg/MyRepo");
        assert_eq!(branch, "main");
    }

    #[test]
    #[serial]
    fn resolve_explicit_repo_infers_branch_from_matching_remote() {
        // --repo matches "upstream", --default-branch omitted:
        // branch should be inferred from upstream's HEAD ("develop"),
        // not origin's ("master").
        let dir = init_git_repo_with_remotes(&[
            ("origin", "https://github.com/my-fork/MyRepo"),
            ("upstream", "https://github.com/MyOrg/MyRepo"),
        ]);
        let repo = git2::Repository::open(dir.path()).unwrap();
        setup_remote_head_refs(
            &repo,
            dir.path(),
            &[("origin", "master"), ("upstream", "develop")],
        );

        let (repo_name, branch) = run_resolve(
            dir.path(),
            &["upload", "mappings.json", "--repo", "MyOrg/MyRepo"],
        )
        .unwrap();
        assert_eq!(repo_name, "MyOrg/MyRepo");
        assert_eq!(branch, "develop");
    }
}
