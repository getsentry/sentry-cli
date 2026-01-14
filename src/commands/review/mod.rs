//! This module implements the `sentry-cli review` command for AI-powered code review.

use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use clap::{ArgMatches, Command};
use console::style;
use git2::{Diff, DiffFormat, DiffOptions, Oid, Repository};
use serde::{Deserialize, Serialize, Serializer};

use crate::api::{Api, Method};
use crate::utils::vcs::git_repo_remote_url;

const ABOUT: &str = "[EXPERIMENTAL] Review local changes using Sentry AI";
const LONG_ABOUT: &str = "\
[EXPERIMENTAL] Review local changes using Sentry AI.

This command analyzes the most recent commit (HEAD vs HEAD~1) and sends it to \
Sentry's AI-powered code review service for bug prediction.

The base commit must be pushed to the remote repository.";

/// Timeout for the review API request (10 minutes)
const REVIEW_TIMEOUT: Duration = Duration::from_secs(600);

/// Maximum diff size in bytes (500 KB)
const MAX_DIFF_SIZE: usize = 500 * 1024;

/// Serializes git2::Oid as a hex string.
fn serialize_oid<S>(oid: &Oid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&oid.to_string())
}

/// Serializes git2::Diff as a unified diff string, skipping binary files.
fn serialize_diff<S>(diff: &&Diff<'_>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut output = Vec::new();
    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        if !delta.flags().is_binary() {
            output.extend_from_slice(line.content());
        }
        true
    })
    .map_err(serde::ser::Error::custom)?;

    let diff_str = String::from_utf8(output).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&diff_str)
}

#[derive(Serialize)]
struct ReviewRequest<'a> {
    remote_url: String,
    #[serde(serialize_with = "serialize_oid")]
    base_commit_sha: Oid,
    #[serde(serialize_with = "serialize_diff")]
    diff: &'a Diff<'a>,
}

#[derive(Deserialize, Debug)]
struct ReviewResponse {
    predictions: Vec<Prediction>,
}

#[derive(Deserialize, Debug)]
struct Prediction {
    file_path: String,
    line_number: Option<u32>,
    description: String,
    severity: String,
    suggested_fix: Option<String>,
}

pub(super) fn make_command(command: Command) -> Command {
    command.about(ABOUT).long_about(LONG_ABOUT)
}

pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    eprintln!(
        "{}",
        style("[EXPERIMENTAL] This feature is in development.").yellow()
    );

    run_review()
}

fn run_review() -> Result<()> {
    // Open repo at top level - keeps it alive for the entire function
    let repo = Repository::open_from_env()
        .context("Failed to open git repository from current directory")?;

    // Get HEAD commit
    let head = repo
        .head()
        .context("Failed to get HEAD reference")?
        .peel_to_commit()
        .context("Failed to resolve HEAD to a commit")?;

    // Check for merge commit (multiple parents)
    if head.parent_count() > 1 {
        bail!("HEAD is a merge commit. Merge commits are not supported for review.");
    }

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

    // Get remote URL
    let remote_url = git_repo_remote_url(&repo, "origin")
        .or_else(|_| git_repo_remote_url(&repo, "upstream"))
        .context("No remote URL found for 'origin' or 'upstream'")?;

    eprintln!("Analyzing commit... (this may take up to 10 minutes)");

    // Build request with borrowed diff - repo still alive
    let request = ReviewRequest {
        remote_url,
        base_commit_sha: parent.id(),
        diff: &diff,
    };

    // Send request and display results
    let response = send_review_request(&request)?;
    display_results(response);

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

/// Sends the review request to the Sentry API.
fn send_review_request(request: &ReviewRequest<'_>) -> Result<ReviewResponse> {
    let api = Api::current();
    api.authenticated()?;

    let path = "/api/0/bug-prediction/cli/";

    let response = api
        .request(Method::Post, path, None)?
        .with_json_body(request)?
        .with_timeout(REVIEW_TIMEOUT)?
        .send()
        .context("Failed to send review request")?;

    response
        .convert::<ReviewResponse>()
        .context("Failed to parse review response")
}

/// Displays the review results in a human-readable format.
fn display_results(response: ReviewResponse) {
    if response.predictions.is_empty() {
        println!("{}", style("No issues found in this commit.").green());
        return;
    }

    println!(
        "{}",
        style(format!(
            "Found {} potential issue(s):",
            response.predictions.len()
        ))
        .yellow()
        .bold()
    );
    println!();

    for (i, prediction) in response.predictions.iter().enumerate() {
        display_prediction(i + 1, prediction);
    }
}

/// Displays a single prediction in a formatted way.
fn display_prediction(index: usize, prediction: &Prediction) {
    let severity_lower = prediction.severity.to_lowercase();

    let styled = match severity_lower.as_str() {
        "high" => style("[HIGH]".to_owned()).red().bold(),
        "medium" => style("[MEDIUM]".to_owned()).yellow().bold(),
        "low" => style("[LOW]".to_owned()).cyan(),
        _ => style(format!("[{}]", prediction.severity.to_uppercase())).dim(),
    };

    println!("{index}. {styled} {}", prediction.file_path);

    if let Some(line) = prediction.line_number {
        println!("   Line: {line}");
    }

    println!("   {}", prediction.description);

    if let Some(fix) = &prediction.suggested_fix {
        println!("   {}: {fix}", style("Suggested fix").green());
    }

    println!();
}
