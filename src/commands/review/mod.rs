//! This module implements the `sentry-cli review` command for AI-powered code review.

use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use clap::{ArgMatches, Args, Command, Parser as _};
use console::style;
use git2::{DiffFormat, DiffOptions, Repository};
use serde::{Deserialize, Serialize};

use crate::api::{Api, Method};
use crate::commands::derive_parser::{SentryCLI, SentryCLICommand};
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

#[derive(Args)]
pub(super) struct ReviewArgs {
    // No additional args for PoC - reviews HEAD vs HEAD~1
}

#[derive(Serialize)]
struct ReviewRequest {
    remote_url: String,
    base_commit_sha: String,
    diff: String,
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
    let SentryCLICommand::Review(_) = SentryCLI::parse().command else {
        unreachable!("expected review command");
    };

    eprintln!(
        "{}",
        style("[EXPERIMENTAL] This feature is in development.").yellow()
    );

    run_review()
}

fn run_review() -> Result<()> {
    let (remote_url, base_sha, diff) = get_review_data()?;

    if diff.trim().is_empty() {
        bail!("No changes found between HEAD and HEAD~1");
    }

    if diff.len() > MAX_DIFF_SIZE {
        bail!(
            "Diff size ({} bytes) exceeds maximum allowed size ({MAX_DIFF_SIZE} bytes)",
            diff.len()
        );
    }

    eprintln!("Analyzing commit... (this may take up to 10 minutes)");

    let response = send_review_request(remote_url, base_sha, diff)?;
    display_results(response);

    Ok(())
}

/// Extracts git diff and metadata from the repository.
fn get_review_data() -> Result<(String, String, String)> {
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

    // Get HEAD~1 (parent) commit
    let parent = head
        .parent(0)
        .context("HEAD has no parent commit - cannot review initial commit")?;
    let base_sha = parent.id().to_string();

    // Get trees for both commits
    let head_tree = head.tree().context("Failed to get HEAD tree")?;
    let parent_tree = parent.tree().context("Failed to get parent tree")?;

    // Generate unified diff, excluding binary files
    let mut diff_opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), Some(&mut diff_opts))
        .context("Failed to generate diff")?;

    let diff_string = generate_diff_string(&diff)?;

    // Get remote URL (prefer origin)
    let remote_url = git_repo_remote_url(&repo, "origin")
        .or_else(|_| git_repo_remote_url(&repo, "upstream"))
        .context("No remote URL found for 'origin' or 'upstream'")?;

    Ok((remote_url, base_sha, diff_string))
}

/// Generates a diff string from a git2::Diff, skipping binary files.
fn generate_diff_string(diff: &git2::Diff) -> Result<String> {
    let mut diff_output = Vec::new();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        // Skip binary files
        if delta.flags().is_binary() {
            return true;
        }

        diff_output.extend_from_slice(line.content());
        true
    })
    .context("Failed to print diff")?;

    String::from_utf8(diff_output).context("Diff contains invalid UTF-8")
}

/// Sends the review request to the Sentry API.
fn send_review_request(
    remote_url: String,
    base_sha: String,
    diff: String,
) -> Result<ReviewResponse> {
    let api = Api::current();
    api.authenticated()?;

    let request_body = ReviewRequest {
        remote_url,
        base_commit_sha: base_sha,
        diff,
    };

    let path = "/api/0/bug-prediction/cli/";

    let response = api
        .request(Method::Post, path, None)?
        .with_json_body(&request_body)?
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

    response
        .predictions
        .iter()
        .enumerate()
        .for_each(|(i, prediction)| {
            display_prediction(i + 1, prediction);
        });
}

/// Displays a single prediction in a formatted way.
fn display_prediction(index: usize, prediction: &Prediction) {
    let severity_label = match prediction.severity.to_lowercase().as_str() {
        "high" => "[HIGH]".to_owned(),
        "medium" => "[MEDIUM]".to_owned(),
        "low" => "[LOW]".to_owned(),
        _ => format!("[{}]", prediction.severity.to_uppercase()),
    };

    let severity_styled = match prediction.severity.to_lowercase().as_str() {
        "high" => style(severity_label).red().bold(),
        "medium" => style(severity_label).yellow().bold(),
        "low" => style(severity_label).cyan(),
        _ => style(severity_label).dim(),
    };

    println!("{index}. {severity_styled} {}", prediction.file_path);

    if let Some(line) = prediction.line_number {
        println!("   Line: {line}");
    }

    println!("   {}", prediction.description);

    if let Some(ref fix) = prediction.suggested_fix {
        println!("   {}: {fix}", style("Suggested fix").green());
    }

    println!();
}
