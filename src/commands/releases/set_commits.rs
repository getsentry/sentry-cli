use anyhow::{bail, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use lazy_static::lazy_static;
use regex::Regex;

use crate::api::{Api, NewRelease, NoneReleaseInfo, OptionalReleaseInfo, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;
use crate::utils::vcs::{
    find_heads, generate_patch_set, get_commits_from_git, get_repo_from_remote, CommitSpec,
};

pub fn make_command(command: Command) -> Command {
    command
        .about("Set commits of a release.")
        .allow_hyphen_values(true)
        .version_arg()
        .arg(Arg::new("clear")
            .long("clear")
            .help("Clear all current commits from the release."))
        .arg(Arg::new("auto")
            .long("auto")
            .help("Enable completely automated commit management.{n}\
                    This requires that the command is run from within a git repository.  \
                    sentry-cli will then automatically find remotely configured \
                    repositories and discover commits."))
        .arg(Arg::new("ignore-missing")
            .long("ignore-missing")
            .help("When the flag is set and the previous release commit was not found in the repository, \
                    will create a release with the default commits count (or the one specified with `--initial-depth`) \
                    instead of failing the command."))
        .arg(Arg::new("local")
            .conflicts_with_all(&["auto", "clear", "commits", ])
            .long("local")
            .help("Set commits of a release from local git.{n}\
                    This requires that the command is run from within a git repository.  \
                    sentry-cli will then automatically find remotely configured \
                    repositories and discover commits."))
        .arg(Arg::new("initial-depth")
            .conflicts_with("auto")
            .long("initial-depth")
            .value_name("INITIAL DEPTH")
            .value_parser(clap::value_parser!(usize))
            .help("Set the number of commits of the initial release. The default is 20."))
        .arg(Arg::new("commits")
            .long("commit")
            .short('c')
            .value_name("SPEC")
            .action(ArgAction::Append)
            .help("Defines a single commit for a repo as \
                    identified by the repo name in the remote Sentry config. \
                    If no commit has been specified sentry-cli will attempt \
                    to auto discover that repository in the local git repo \
                    and then use the HEAD commit.  This will either use the \
                    current git repository or attempt to auto discover a \
                    submodule with a compatible URL.\n\n\
                    The value can be provided as `REPO` in which case sentry-cli \
                    will auto-discover the commit based on reachable repositories. \
                    Alternatively it can be provided as `REPO#PATH` in which case \
                    the current commit of the repository at the given PATH is \
                    assumed.  To override the revision `@REV` can be appended \
                    which will force the revision to a certain value."))
        // Legacy flag that has no effect, left hidden for backward compatibility
        .arg(Arg::new("ignore-empty")
            .long("ignore-empty").hide(true))
}

fn strip_sha(sha: &str) -> &str {
    lazy_static! {
        static ref SHA_RE: Regex = Regex::new(r"^[a-fA-F0-9]{40}$").unwrap();
    }
    if SHA_RE.is_match(sha) {
        &sha[..12]
    } else {
        sha
    }
}
pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.get_one::<String>("version").unwrap();
    let org = config.get_org(matches)?;
    let repos = api.list_organization_repos(&org)?;
    let mut commit_specs = vec![];

    let heads = if repos.is_empty() {
        None
    } else if matches.contains_id("auto") {
        let commits = find_heads(None, &repos, Some(config.get_cached_vcs_remote()))?;
        if commits.is_empty() {
            None
        } else {
            Some(commits)
        }
    } else if matches.contains_id("clear") {
        Some(vec![])
    } else if matches.contains_id("local") {
        None
    } else {
        if let Some(commits) = matches.get_many::<String>("commits") {
            for spec in commits {
                let commit_spec = CommitSpec::parse(spec)?;
                if repos.iter().any(|r| r.name == commit_spec.repo) {
                    commit_specs.push(commit_spec);
                } else {
                    bail!("Unknown repo '{}'", commit_spec.repo);
                }
            }
        }
        let commits = find_heads(
            Some(commit_specs),
            &repos,
            Some(config.get_cached_vcs_remote()),
        )?;
        if commits.is_empty() {
            None
        } else {
            Some(commits)
        }
    };

    // make sure the release exists if projects are given
    if let Ok(projects) = config.get_projects(matches) {
        api.new_release(
            &org,
            &NewRelease {
                version: version.into(),
                projects,
                ..Default::default()
            },
        )?;
    }

    if let Some(heads) = heads {
        if heads.is_empty() {
            println!("Clearing commits for release.");
        } else {
            let mut table = Table::new();
            table.title_row().add("Repository").add("Revision");
            for commit in &heads {
                let row = table.add_row();
                row.add(&commit.repo);
                if let Some(ref prev_rev) = commit.prev_rev {
                    row.add(format!(
                        "{} -> {}",
                        strip_sha(prev_rev),
                        strip_sha(&commit.rev)
                    ));
                } else {
                    row.add(strip_sha(&commit.rev));
                }
            }
            table.print();
        }
        api.set_release_refs(&org, version, heads)?;
    } else {
        let default_count = matches
            .get_one::<usize>("initial-depth")
            .copied()
            .unwrap_or(20);

        if matches.contains_id("auto") {
            println!("Could not determine any commits to be associated with a repo-based integration. Proceeding to find commits from local git tree.");
        }
        // Get the commit of the most recent release.
        let prev_commit = match api.get_previous_release_with_commits(&org, version)? {
            OptionalReleaseInfo::Some(prev) => prev.last_commit.map(|c| c.id).unwrap_or_default(),
            OptionalReleaseInfo::None(NoneReleaseInfo {}) => String::new(),
        };

        // Find and connect to local git.
        let repo = git2::Repository::open_from_env()?;

        // Parse the git url.
        let remote = config.get_cached_vcs_remote();
        let parsed = get_repo_from_remote(&remote);
        let ignore_missing = matches.contains_id("ignore-missing");
        // Fetch all the commits upto the `prev_commit` or return the default (20).
        // Will return a tuple of Vec<GitCommits> and the `prev_commit` if it exists in the git tree.
        let (commit_log, prev_commit) =
            get_commits_from_git(&repo, &prev_commit, default_count, ignore_missing)?;

        // Calculate the diff for each commit in the Vec<GitCommit>.
        let commits = generate_patch_set(&repo, commit_log, prev_commit, &parsed)?;

        if commits.is_empty() {
            println!("No commits found. Leaving release alone. If you believe there should be some, change commits range or initial depth and try again.");
            return Ok(());
        }

        api.update_release(
            &config.get_org(matches)?,
            version,
            &UpdatedRelease {
                commits: Some(commits),
                ..Default::default()
            },
        )?;

        println!("Success! Set commits for release {version}");
    }

    Ok(())
}
