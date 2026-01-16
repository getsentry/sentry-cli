//! This module implements the `sentry-cli review` command for AI-powered code review.

use anyhow::{bail, Context as _, Result};
use clap::{ArgMatches, Args, Command, Parser as _};
use console::style;
use git2::{Diff, DiffOptions, Repository};

use super::derive_parser::{SentryCLI, SentryCLICommand};
use crate::api::{Api, ReviewRepository, ReviewRequest};
use crate::config::Config;
use crate::utils::vcs::{get_repo_from_remote, git_repo_remote_url};

const ABOUT: &str = "[EXPERIMENTAL] Review local changes using Sentry AI";
const LONG_ABOUT: &str = "\
[EXPERIMENTAL] Review local changes using Sentry AI.

This command analyzes the most recent commit (HEAD vs HEAD~1) and sends it to \
Sentry's AI-powered code review service for bug prediction.

The base commit must be pushed to the remote repository.";

/// Maximum diff size in bytes (500 KB)
const MAX_DIFF_SIZE: usize = 500 * 1024;

#[derive(Args)]
#[command(about = ABOUT, long_about = LONG_ABOUT, hide = true)]
pub(super) struct ReviewArgs {
    #[arg(short = 'o', long = "org")]
    #[arg(help = "The organization ID or slug.")]
    org: Option<String>,
}

pub(super) fn make_command(command: Command) -> Command {
    ReviewArgs::augment_args(command)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::Review(args) = SentryCLI::parse().command else {
        unreachable!("expected review command");
    };

    eprintln!(
        "{}",
        style("[EXPERIMENTAL] This feature is in development.").yellow()
    );

    run_review(args)
}

fn run_review(args: ReviewArgs) -> Result<()> {
    // Resolve organization
    let config = Config::current();
    let (default_org, _) = config.get_org_and_project_defaults();
    let org = args.org.as_ref().or(default_org.as_ref()).ok_or_else(|| {
        anyhow::anyhow!(
            "No organization specified. Please specify an organization using the --org argument."
        )
    })?;

    // Open repo at top level - keeps it alive for the entire function
    let repo = Repository::open_from_env()
        .context("Failed to open git repository from current directory")?;

    // Get HEAD reference for current branch name
    let head_ref = repo.head().context("Failed to get HEAD reference")?;
    let current_branch = head_ref.shorthand().map(String::from);

    // Get HEAD commit
    let head = head_ref
        .peel_to_commit()
        .context("Failed to resolve HEAD to a commit")?;

    // Check for merge commit (multiple parents)
    if head.parent_count() > 1 {
        bail!("HEAD is a merge commit. Merge commits are not supported for review.");
    }

    // Get commit message
    let commit_message = head.message().map(ToOwned::to_owned);

    // Get parent commit
    let parent = head
        .parent(0)
        .context("HEAD has no parent commit - cannot review initial commit")?;

    // Get trees for diff
    let head_tree = head.tree().context("Failed to get HEAD tree")?;
    let parent_tree = parent.tree().context("Failed to get parent tree")?;

    // Generate diff (borrows from repo)
    let mut diff_opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), Some(&mut diff_opts))
        .context("Failed to generate diff")?;

    // Validate diff
    validate_diff(&diff)?;

    // Get remote URL and extract repo name
    let remote_url = git_repo_remote_url(&repo, "origin")
        .or_else(|_| git_repo_remote_url(&repo, "upstream"))
        .context("No remote URL found for 'origin' or 'upstream'")?;
    let repo_name = get_repo_from_remote(&remote_url);

    eprintln!("Analyzing commit... (this may take up to 10 minutes)");

    // Build request with borrowed diff - repo still alive
    let request = ReviewRequest {
        repository: ReviewRepository {
            name: repo_name,
            base_commit_sha: parent.id(),
        },
        diff: &diff,
        current_branch,
        commit_message,
    };

    // Send request and output raw JSON
    let response = Api::current()
        .authenticated()
        .context("Authentication required for review")?
        .review_code(org, &request)
        .context("Failed to get review results")?;

    // Output raw JSON for agentic workflow consumption
    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}

/// Validates the diff meets requirements.
fn validate_diff(diff: &Diff<'_>) -> Result<()> {
    let stats = diff.stats().context("Failed to get diff stats")?;

    if stats.files_changed() == 0 {
        bail!("No changes found between HEAD and HEAD~1");
    }

    // Estimate size by summing insertions and deletions (rough approximation)
    let estimated_size = (stats.insertions() + stats.deletions()) * 80; // ~80 chars per line
    if estimated_size > MAX_DIFF_SIZE {
        bail!("Diff is too large (estimated {estimated_size} bytes, max {MAX_DIFF_SIZE} bytes)");
    }

    Ok(())
}
