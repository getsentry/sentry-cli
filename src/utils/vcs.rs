#![expect(clippy::unwrap_used, reason = "contains legacy code which uses unwrap")]

use std::fmt;
use std::path::PathBuf;

use anyhow::{bail, format_err, Error, Result};
use chrono::{DateTime, FixedOffset, TimeZone as _};
use git2::{Commit, Repository, Time};
use if_chain::if_chain;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use regex::Regex;
use serde::Deserialize;

use crate::api::{GitCommit, PatchSet, Ref, Repo};

/// Represents the structure of a GitHub Actions event payload for pull requests
#[derive(Deserialize, Debug)]
struct GitHubEventPayload {
    pull_request: Option<GitHubPullRequest>,
}

/// Represents the pull request object in the GitHub event payload
#[derive(Deserialize, Debug)]
struct GitHubPullRequest {
    head: GitHubHead,
}

/// Represents the head object in the GitHub pull request
#[derive(Deserialize, Debug)]
struct GitHubHead {
    sha: String,
}

#[derive(Copy, Clone)]
pub enum GitReference<'a> {
    Commit(git2::Oid),
    Symbolic(&'a str),
}

impl fmt::Display for GitReference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GitReference::Commit(ref c) => write!(f, "{c}"),
            GitReference::Symbolic(ref s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub struct CommitSpec {
    pub repo: String,
    pub path: Option<PathBuf>,
    pub rev: String,
    pub prev_rev: Option<String>,
}

impl fmt::Display for CommitSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", &self.repo, &self.rev)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct VcsUrl {
    pub provider: String,
    pub id: String,
}

macro_rules! log_match {
    ($ex:expr) => {{
        let val = $ex;
        info!("  -> found matching revision {}", val);
        val
    }};
}

fn parse_rev_range(rng: &str) -> (Option<String>, String) {
    if rng.is_empty() {
        return (None, "HEAD".into());
    }
    let mut iter = rng.rsplitn(2, "..");
    let rev = iter.next().unwrap_or("HEAD");
    (iter.next().map(str::to_owned), rev.to_owned())
}

impl CommitSpec {
    pub fn parse(s: &str) -> Result<CommitSpec> {
        lazy_static! {
            static ref SPEC_RE: Regex = Regex::new(r"^([^@#]+)(?:#([^@]+))?(?:@(.+))?$").unwrap();
        }
        if let Some(caps) = SPEC_RE.captures(s) {
            let (prev_rev, rev) = parse_rev_range(caps.get(3).map(|x| x.as_str()).unwrap_or(""));
            Ok(CommitSpec {
                repo: caps[1].to_string(),
                path: caps.get(2).map(|x| PathBuf::from(x.as_str())),
                rev,
                prev_rev,
            })
        } else {
            bail!("Could not parse commit spec '{}'", s)
        }
    }

    pub fn reference(&self) -> GitReference<'_> {
        if let Ok(oid) = git2::Oid::from_str(&self.rev) {
            GitReference::Commit(oid)
        } else {
            GitReference::Symbolic(&self.rev)
        }
    }

    pub fn prev_reference(&self) -> Option<GitReference<'_>> {
        self.prev_rev.as_ref().map(|rev| {
            if let Ok(oid) = git2::Oid::from_str(rev) {
                GitReference::Commit(oid)
            } else {
                GitReference::Symbolic(rev)
            }
        })
    }
}

fn strip_git_suffix(s: &str) -> &str {
    s.trim_end_matches(".git")
}

impl VcsUrl {
    pub fn parse(url: &str) -> VcsUrl {
        Self::parse_preserve_case(url).into_lowercase()
    }

    pub fn parse_preserve_case(url: &str) -> VcsUrl {
        lazy_static! {
            static ref GIT_URL_RE: Regex =
                Regex::new(r"^(?:(?:git\+)?(?:git|ssh|https?))://(?:[^@]+@)?([^/]+)/(.+)$")
                    .unwrap();
            static ref GIT_SSH_RE: Regex = Regex::new(r"^(?:[^@]+@)?([^/]+):(.+)$").unwrap();
        }

        if let Some(caps) = GIT_URL_RE.captures(url) {
            return VcsUrl::from_git_parts(&caps[1], &caps[2]);
        }

        if let Some(caps) = GIT_SSH_RE.captures(url) {
            return VcsUrl::from_git_parts(&caps[1], &caps[2]);
        }

        VcsUrl {
            provider: "".into(),
            id: url.into(),
        }
    }

    fn into_lowercase(mut self) -> VcsUrl {
        self.id = self.id.to_lowercase();
        self
    }

    fn from_git_parts(host: &str, path: &str) -> VcsUrl {
        // Azure Devops has multiple domains and multiple URL styles for the
        // various different API versions.
        lazy_static! {
            static ref AZUREDEV_DOMAIN_RE: Regex =
                Regex::new(r"^(?:ssh\.)?(dev.azure.com)$").unwrap();
            static ref AZUREDEV_VERSION_PATH_RE: Regex =
                Regex::new(r"^v3/([^/]+)/([^/]+)").unwrap();
            static ref VS_DOMAIN_RE: Regex = Regex::new(r"^([^.]+)\.visualstudio.com$").unwrap();
            static ref VS_GIT_PATH_RE: Regex = Regex::new(r"^_git/(.+?)(?:\.git)?$").unwrap();
            static ref VS_TRAILING_GIT_PATH_RE: Regex = Regex::new(r"^(.+?)/_git").unwrap();
            static ref HOST_WITH_PORT: Regex = Regex::new(r"(.*):\d+$").unwrap();
            static ref GCB_GIT_PATH_RE: Regex =
                Regex::new(r"^p/.+/r/github_(.+?)_(.+?)(?:\.git)?$").unwrap();
            static ref BITBUCKET_SERVER_PATH_RE: Regex =
                Regex::new(r"projects/(.+)/repos/(.+)/browse").unwrap();
        }
        static GCB_DOMAIN: &str = "source.developers.google.com";

        if let Some(caps) = HOST_WITH_PORT.captures(host) {
            return VcsUrl::from_git_parts(&caps[1], path);
        }

        if let Some(caps) = VS_DOMAIN_RE.captures(host) {
            let username = &caps[1];
            if let Some(caps) = VS_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: host.to_lowercase(),
                    id: format!("{username}/{}", &caps[1]),
                };
            }
            if let Some(caps) = VS_TRAILING_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: host.to_lowercase(),
                    id: caps[1].to_string(),
                };
            }
        }

        if let Some(caps) = AZUREDEV_DOMAIN_RE.captures(host) {
            let hostname = &caps[1];
            if let Some(caps) = AZUREDEV_VERSION_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: hostname.into(),
                    id: format!("{}/{}", &caps[1], &caps[2]),
                };
            }
            if let Some(caps) = VS_TRAILING_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: hostname.to_lowercase(),
                    id: caps[1].to_string(),
                };
            }
        }

        if host == GCB_DOMAIN {
            if let Some(caps) = GCB_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: host.into(),
                    id: format!("{}/{}", &caps[1], &caps[2]),
                };
            }
        }

        if let Some(caps) = BITBUCKET_SERVER_PATH_RE.captures(path) {
            return VcsUrl {
                provider: host.to_lowercase(),
                id: format!("{}/{}", &caps[1], &caps[2]),
            };
        }

        VcsUrl {
            provider: host.to_lowercase(),
            id: strip_git_suffix(path).to_owned(),
        }
    }
}

fn extract_provider_name(host: &str) -> &str {
    let trimmed = host.trim_end_matches('.');
    trimmed.rsplit('.').nth(1).unwrap_or(trimmed)
}

fn is_matching_url(a: &str, b: &str) -> bool {
    VcsUrl::parse(a) == VcsUrl::parse(b)
}

pub fn get_repo_from_remote(repo: &str) -> String {
    let obj = VcsUrl::parse(repo);
    obj.id
}

/// Like get_repo_from_remote but preserves the original case of the repository name.
/// This is used specifically for build upload where case preservation is important.
pub fn get_repo_from_remote_preserve_case(repo: &str) -> String {
    let obj = VcsUrl::parse_preserve_case(repo);
    obj.id
}

pub fn get_provider_from_remote(remote: &str) -> String {
    let obj = VcsUrl::parse(remote);
    extract_provider_name(&obj.provider).to_owned()
}

pub fn git_repo_remote_url(
    repo: &git2::Repository,
    cached_remote: &str,
) -> Result<String, git2::Error> {
    let remote = repo.find_remote(cached_remote)?;
    remote
        .url()
        .map(|url| url.to_owned())
        .ok_or_else(|| git2::Error::from_str("No remote URL found"))
}

pub fn git_repo_head_ref(repo: &git2::Repository) -> Result<String> {
    let head = repo.head()?;

    // Only return a reference name if we're not in a detached HEAD state
    // In detached HEAD state, head.shorthand() returns "HEAD" which is not a valid branch name
    if head.is_branch() {
        head.shorthand()
            .map(|s| s.to_owned())
            .ok_or_else(|| anyhow::anyhow!("No HEAD reference found"))
    } else {
        // In detached HEAD state, return an error to indicate no valid branch reference
        Err(anyhow::anyhow!(
            "HEAD is detached - no branch reference available"
        ))
    }
}

pub fn git_repo_base_ref(repo: &git2::Repository, remote_name: &str) -> Result<String> {
    // Get the current HEAD commit
    let head_commit = repo.head()?.peel_to_commit()?;

    // Try to find the remote tracking branch
    let remote_branch_name = format!("refs/remotes/{remote_name}/HEAD");
    let remote_ref = repo.find_reference(&remote_branch_name).map_err(|e| {
        anyhow::anyhow!(
            "Could not find remote tracking branch for {}: {}",
            remote_name,
            e
        )
    })?;

    find_merge_base_ref(repo, &head_commit, &remote_ref)
}

fn find_merge_base_ref(
    repo: &git2::Repository,
    head_commit: &git2::Commit,
    remote_ref: &git2::Reference,
) -> Result<String> {
    let remote_commit = remote_ref.peel_to_commit()?;
    let merge_base_oid = repo.merge_base(head_commit.id(), remote_commit.id())?;

    // Return the merge-base commit SHA as the base reference
    let merge_base_sha = merge_base_oid.to_string();
    debug!(
        "Found merge-base commit as base reference: {}",
        merge_base_sha
    );
    Ok(merge_base_sha)
}

/// Like git_repo_base_repo_name but preserves the original case of the repository name.
/// This is used specifically for build upload where case preservation is important.
pub fn git_repo_base_repo_name_preserve_case(repo: &git2::Repository) -> Result<Option<String>> {
    let remotes = repo.remotes()?;
    let remote_names: Vec<&str> = remotes.iter().flatten().collect();

    if remote_names.is_empty() {
        warn!("No remotes found in repository");
        return Ok(None);
    }

    // Prefer "upstream" if it exists, then "origin", otherwise use the first one
    let chosen_remote = if remote_names.contains(&"upstream") {
        "upstream"
    } else if remote_names.contains(&"origin") {
        "origin"
    } else {
        remote_names[0]
    };

    match git_repo_remote_url(repo, chosen_remote) {
        Ok(remote_url) => {
            debug!("Found remote '{}': {}", chosen_remote, remote_url);
            let repo_name = get_repo_from_remote_preserve_case(&remote_url);
            Ok(Some(repo_name))
        }
        Err(e) => {
            warn!("Could not get URL for remote '{}': {}", chosen_remote, e);
            Ok(None)
        }
    }
}

/// Attempts to get the PR number from GitHub Actions environment variables.
/// Returns the PR number if running in a GitHub Actions pull request environment.
pub fn get_github_pr_number() -> Option<u32> {
    let github_ref = std::env::var("GITHUB_REF").ok()?;
    let event_name = std::env::var("GITHUB_EVENT_NAME").ok()?;

    if event_name != "pull_request" {
        debug!("Not running in pull_request event, got: {}", event_name);
        return None;
    }

    let pr_number_str = github_ref.strip_prefix("refs/pull/")?;
    debug!("Extracted PR reference: {}", pr_number_str);

    let pr_number_str = pr_number_str.split('/').next()?;
    debug!("Parsing PR number from: {}", pr_number_str);

    let pr_number = pr_number_str.parse().ok()?;
    debug!("Auto-detected PR number from GitHub Actions: {}", pr_number);
    Some(pr_number)
}

/// Attempts to get the base branch from GitHub Actions environment variables.
/// Returns the base branch name if running in a GitHub Actions pull request environment.
pub fn get_github_base_ref() -> Option<String> {
    let event_name = std::env::var("GITHUB_EVENT_NAME").ok()?;

    if event_name != "pull_request" {
        debug!("Not running in pull_request event, got: {}", event_name);
        return None;
    }

    let base_ref = std::env::var("GITHUB_BASE_REF").ok()?;
    debug!("Auto-detected base ref from GitHub Actions: {}", base_ref);
    Some(base_ref)
}

fn find_reference_url(repo: &str, repos: &[Repo]) -> Result<Option<String>> {
    let mut non_git = false;
    for configured_repo in repos {
        if configured_repo.name != repo {
            continue;
        }

        match configured_repo.provider.id.as_str() {
            "git"
            | "github"
            | "bitbucket"
            | "visualstudio"
            | "google"
            | "integrations:github"
            | "integrations:github_enterprise"
            | "integrations:gitlab"
            | "integrations:bitbucket"
            | "integrations:bitbucket_server"
            | "integrations:vsts" => {
                if let Some(ref url) = configured_repo.url {
                    debug!("  Got reference URL for repo {}: {}", repo, url);
                    return Ok(Some(url.clone()));
                }
            }
            _ => {
                debug!("  unknown repository {} skipped", configured_repo);
                non_git = true;
            }
        }
    }

    if non_git {
        Ok(None)
    } else {
        bail!("Could not find matching repository for {}", repo);
    }
}

fn find_matching_rev(
    reference: GitReference<'_>,
    spec: &CommitSpec,
    repos: &[Repo],
    disable_discovery: bool,
    remote_name: Option<String>,
) -> Result<Option<String>> {
    info!("Resolving {} ({})", &reference, spec);

    let r = match reference {
        GitReference::Commit(commit) => {
            return Ok(Some(log_match!(commit.to_string())));
        }
        GitReference::Symbolic(r) => r,
    };

    let (repo, discovery) = if let Some(ref path) = spec.path {
        (git2::Repository::open(path)?, false)
    } else {
        (git2::Repository::open_from_env()?, !disable_discovery)
    };

    match find_reference_url(&spec.repo, repos)? {
        None => Ok(None),
        Some(reference_url) => {
            debug!("  Looking for reference URL {}", &reference_url);

            // direct reference in root repository found.  If we are in discovery
            // mode we want to also check for matching URLs.
            if_chain! {
                if let Ok(remote) = repo.find_remote(&remote_name.unwrap_or_else(|| "origin".to_owned()));
                if let Some(url) = remote.url();
                then {
                    if !discovery || is_matching_url(url, &reference_url) {
                        debug!("  found match: {} == {}, {:?}", url, &reference_url, r);
                        let head = repo.revparse_single(r)?;
                        if let Some(tag) = head.as_tag(){
                            if let Ok(tag_commit) = tag.target() {
                                return Ok(Some(log_match!(tag_commit.id().to_string())));
                            }
                        }
                        return Ok(Some(log_match!(head.id().to_string())));
                    } else {
                        debug!("  not a match: {} != {}", url, &reference_url);
                    }
                }
            }
            if let Ok(submodule_match) = find_matching_submodule(r, reference_url, repo) {
                return Ok(submodule_match);
            }
            info!("  -> no matching revision found");
            Ok(None)
        }
    }
}

fn find_matching_submodule(
    r: &str,
    reference_url: String,
    repo: git2::Repository,
) -> Result<Option<String>> {
    // in discovery mode we want to find that repo in associated submodules.
    for submodule in repo.submodules()? {
        if let Some(submodule_url) = submodule.url() {
            debug!("  found submodule with URL {}", submodule_url);
            if is_matching_url(submodule_url, &reference_url) {
                debug!(
                    "  found submodule match: {} == {}",
                    submodule_url, &reference_url
                );
                // heads on submodules is easier so let's start with that
                // because that does not require the submodule to be
                // checked out.
                if r == "HEAD" {
                    if let Some(head_oid) = submodule.head_id() {
                        return Ok(Some(log_match!(head_oid.to_string())));
                    }
                }

                // otherwise we need to open the submodule which requires
                // it to be checked out.
                if let Ok(subrepo) = submodule.open() {
                    let head = subrepo.revparse_single(r)?;
                    return Ok(Some(log_match!(head.id().to_string())));
                }
            } else {
                debug!(
                    "  not a submodule match: {} != {}",
                    submodule_url, &reference_url
                );
            }
        }
    }
    Ok(None)
}

fn find_matching_revs(
    spec: &CommitSpec,
    repos: &[Repo],
    disable_discovery: bool,
    remote_name: Option<String>,
) -> Result<(Option<String>, String)> {
    fn error(r: GitReference<'_>, repo: &str) -> Error {
        format_err!(
            "Could not find commit '{}' for '{}'. If you do not have local \
             checkouts of the repositories in question referencing tags or \
             other references will not work and you need to refer to \
             revisions explicitly.",
            r,
            repo
        )
    }

    let rev = if let Some(rev) = find_matching_rev(
        spec.reference(),
        spec,
        repos,
        disable_discovery,
        remote_name.clone(),
    )? {
        rev
    } else {
        return Err(error(spec.reference(), &spec.repo));
    };

    let prev_rev = if let Some(rev) = spec.prev_reference() {
        if let Some(rv) = find_matching_rev(rev, spec, repos, disable_discovery, remote_name)? {
            Some(rv)
        } else {
            return Err(error(rev, &spec.repo));
        }
    } else {
        None
    };

    Ok((prev_rev, rev))
}

pub fn find_head() -> Result<String> {
    if let Ok(event_path) = std::env::var("GITHUB_EVENT_PATH") {
        if let Ok(content) = std::fs::read_to_string(&event_path) {
            if let Some(pr_head_sha) = extract_pr_head_sha_from_event(&content) {
                debug!(
                    "Using GitHub Actions PR head SHA from event payload: {}",
                    pr_head_sha
                );
                return Ok(pr_head_sha);
            }
        }
    }

    let repo = git2::Repository::open_from_env()?;
    let head = repo.revparse_single("HEAD")?;
    Ok(head.id().to_string())
}

/// Extracts the PR head SHA from GitHub Actions event payload JSON.
/// Returns None if not a PR event or if SHA cannot be extracted.
fn extract_pr_head_sha_from_event(json_content: &str) -> Option<String> {
    let payload: GitHubEventPayload = match serde_json::from_str(json_content) {
        Ok(payload) => payload,
        Err(_) => {
            debug!("Failed to parse GitHub event payload as JSON");
            return None;
        }
    };

    Some(payload.pull_request?.head.sha)
}

/// Given commit specs, repos and remote_name this returns a list of head
/// commits from it.
pub fn find_heads(
    specs: Option<Vec<CommitSpec>>,
    repos: &[Repo],
    remote_name: Option<String>,
) -> Result<Vec<Ref>> {
    let mut rv = vec![];

    // if commit specs were explicitly provided find head commits with
    // limited amounts of magic.
    if let Some(specs) = specs {
        for spec in &specs {
            let (prev_rev, rev) =
                find_matching_revs(spec, repos, specs.len() == 1, remote_name.clone())?;
            rv.push(Ref {
                repo: spec.repo.clone(),
                rev,
                prev_rev,
            });
        }

    // otherwise apply all the magic available
    } else {
        for repo in repos {
            let spec = CommitSpec {
                repo: repo.name.clone(),
                path: None,
                rev: "HEAD".into(),
                prev_rev: None,
            };
            if let Some(rev) =
                find_matching_rev(spec.reference(), &spec, repos, false, remote_name.clone())?
            {
                rv.push(Ref {
                    repo: repo.name.clone(),
                    rev,
                    prev_rev: None,
                });
            }
        }
    }

    Ok(rv)
}

// Get commits from git history upto previous commit.
// Returns a tuple of Vec<GitCommits> and the `prev_commit` if it exists in the git tree.
pub fn get_commits_from_git<'a>(
    repo: &'a Repository,
    prev_commit: &str,
    default_count: usize,
    ignore_missing: bool,
) -> Result<(Vec<Commit<'a>>, Option<Commit<'a>>)> {
    match git2::Oid::from_str(prev_commit) {
        Ok(prev) => {
            let mut found = false;
            let mut revwalk = repo.revwalk()?;
            revwalk.push_head()?;
            let mut result: Vec<Commit> = revwalk
                .take_while(|id| match id {
                    Ok(id) => {
                        if found {
                            return false;
                        }
                        if prev == *id {
                            found = true;
                        }
                        true
                    }
                    _ => true,
                })
                .filter_map(move |id: Result<git2::Oid, git2::Error>| {
                    repo.find_commit(id.ok()?).ok()
                })
                .collect();

            // If there is a previous commit but cannot find it in git history
            if !found {
                // Create a new release with default count if `--ignore-missing` is present
                if ignore_missing {
                    println!(
                        "Could not find the SHA of the previous release in the git history. Skipping previous release and creating a new one with {default_count} commits."
                    );
                    return get_default_commits_from_git(repo, default_count);
                // Or throw an error and point to the right solution otherwise.
                } else {
                    return Err(format_err!(
                        "Could not find the SHA of the previous release in the git history. If you limit the clone depth, try to increase it. \
                        Otherwise, it means that the commit we are looking for was amended or squashed and cannot be retrieved. \
                        Use --ignore-missing flag to skip it and create a new release with the default commits count.",
                    ));
                }
            }
            let prev = result.pop();
            Ok((result, prev))
        }
        Err(_) => {
            // If there is no previous commit, return the default number of commits
            println!(
                "Could not find the previous commit. Creating a release with {default_count} commits."
            );
            get_default_commits_from_git(repo, default_count)
        }
    }
}

pub fn get_default_commits_from_git(
    repo: &Repository,
    default_count: usize,
) -> Result<(Vec<Commit<'_>>, Option<Commit<'_>>)> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let mut result: Vec<Commit> = revwalk
        .take(default_count + 1)
        .filter_map(move |id: Result<git2::Oid, git2::Error>| repo.find_commit(id.ok()?).ok())
        .collect();

    if result.len() == default_count + 1 {
        let prev = result.pop();
        Ok((result, prev))
    } else {
        Ok((result, None))
    }
}

pub fn generate_patch_set(
    repo: &Repository,
    commits: Vec<Commit>,
    previous: Option<Commit>,
    repository: &str,
) -> Result<Vec<GitCommit>> {
    let mut result = vec![];
    for (index, commit) in commits.iter().enumerate() {
        let mut git_commit = GitCommit {
            id: commit.id().to_string(),
            author_name: commit.author().name().map(|s| s.to_owned()),
            author_email: commit.author().email().map(|s| s.to_owned()),
            message: commit.message().map(|s| s.to_owned()),
            repository: repository.to_owned(),
            timestamp: get_commit_time(commit.author().when()),
            patch_set: vec![],
        };

        let old_tree = if commits.len() > index + 1 {
            Some(commits[index + 1].tree()?)
        } else {
            previous.as_ref().map(|c| c.tree()).transpose()?
        };

        let new_tree = Some(commit.tree()?);
        let diff = repo.diff_tree_to_tree(old_tree.as_ref(), new_tree.as_ref(), None)?;

        diff.print(git2::DiffFormat::NameStatus, |_, _, l| {
            let line = std::str::from_utf8(l.content()).unwrap();
            let mut parsed = line.trim_end().splitn(2, '\t');
            let patch_set = PatchSet {
                ty: parsed.next().unwrap().to_owned(),
                path: parsed.next().unwrap().to_owned(),
            };
            git_commit.patch_set.push(patch_set);

            // Returning false from the callback will terminate the iteration and return an error from this function.
            true
        })?;

        result.push(git_commit)
    }

    Ok(result)
}

pub fn get_commit_time(time: Time) -> DateTime<FixedOffset> {
    FixedOffset::east_opt(time.offset_minutes() * 60)
        .unwrap()
        .timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use {
        crate::api::RepoProvider,
        insta::{assert_debug_snapshot, assert_yaml_snapshot},
        std::fs::File,
        std::io::Write as _,
        std::path::Path,
        std::process::Command,
        tempfile::{tempdir, TempDir},
    };

    use super::*;

    #[test]
    fn test_find_matching_rev_with_lightweight_tag() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        let hash = git_create_tag(dir.path(), "1.9.2", false);

        let reference = GitReference::Symbolic("1.9.2");
        let spec = CommitSpec {
            repo: String::from("getsentry/sentry-cli"),
            path: Some(dir.path().to_path_buf()),
            rev: String::from("1.9.2"),
            prev_rev: Some(String::from("1.9.1")),
        };

        let repos = [Repo {
            id: String::from("1"),
            name: String::from("getsentry/sentry-cli"),
            url: Some(String::from("https://github.com/getsentry/sentry-cli")),
            provider: RepoProvider {
                id: String::from("integrations:github"),
                name: String::from("GitHub"),
            },
            status: String::from("active"),
            date_created: chrono::Utc::now(),
        }];

        let res_with_lightweight_tag = find_matching_rev(reference, &spec, &repos, false, None);
        assert_eq!(res_with_lightweight_tag.unwrap(), Some(hash));
    }

    #[test]
    fn test_find_matching_rev_with_annotated_tag() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        let hash = git_create_tag(dir.path(), "1.9.2-hw", true);

        let reference = GitReference::Symbolic("1.9.2-hw");
        let spec = CommitSpec {
            repo: String::from("getsentry/sentry-cli"),
            path: Some(dir.path().to_path_buf()),
            rev: String::from("1.9.2-hw"),
            prev_rev: Some(String::from("1.9.1")),
        };

        let repos = [Repo {
            id: String::from("1"),
            name: String::from("getsentry/sentry-cli"),
            url: Some(String::from("https://github.com/getsentry/sentry-cli")),
            provider: RepoProvider {
                id: String::from("integrations:github"),
                name: String::from("GitHub"),
            },
            status: String::from("active"),
            date_created: chrono::Utc::now(),
        }];

        let res_with_annotated_tag = find_matching_rev(reference, &spec, &repos, false, None);
        assert_eq!(res_with_annotated_tag.unwrap(), Some(hash));
    }

    #[test]
    fn test_url_parsing() {
        assert_eq!(
            VcsUrl::parse("http://github.com/mitsuhiko/flask"),
            VcsUrl {
                provider: "github.com".into(),
                id: "mitsuhiko/flask".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@github.com:mitsuhiko/flask.git"),
            VcsUrl {
                provider: "github.com".into(),
                id: "mitsuhiko/flask".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("http://bitbucket.org/mitsuhiko/flask"),
            VcsUrl {
                provider: "bitbucket.org".into(),
                id: "mitsuhiko/flask".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@bitbucket.org:mitsuhiko/flask.git"),
            VcsUrl {
                provider: "bitbucket.org".into(),
                id: "mitsuhiko/flask".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse(
                "https://bitbucket.example.com/projects/laurynsentry/repos/helloworld/browse"
            ),
            VcsUrl {
                provider: "bitbucket.example.com".into(),
                id: "laurynsentry/helloworld".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://neilmanvar.visualstudio.com/_git/sentry-demo"),
            VcsUrl {
                provider: "neilmanvar.visualstudio.com".into(),
                id: "neilmanvar/sentry-demo".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://project@mydomain.visualstudio.com/project/repo/_git"),
            VcsUrl {
                provider: "mydomain.visualstudio.com".into(),
                id: "project/repo".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@ssh.dev.azure.com:v3/project/repo/repo"),
            VcsUrl {
                provider: "dev.azure.com".into(),
                id: "project/repo".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@ssh.dev.azure.com:v3/company/Repo%20Online/Repo%20Online"),
            VcsUrl {
                provider: "dev.azure.com".into(),
                id: "company/repo%20online".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://dev.azure.com/project/repo/_git/repo"),
            VcsUrl {
                provider: "dev.azure.com".into(),
                id: "project/repo".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://dev.azure.com/company/Repo%20Online/_git/Repo%20Online"),
            VcsUrl {
                provider: "dev.azure.com".into(),
                id: "company/repo%20online".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://github.myenterprise.com/mitsuhiko/flask.git"),
            VcsUrl {
                provider: "github.myenterprise.com".into(),
                id: "mitsuhiko/flask".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://gitlab.example.com/gitlab-org/gitlab-ce"),
            VcsUrl {
                provider: "gitlab.example.com".into(),
                id: "gitlab-org/gitlab-ce".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@gitlab.example.com:gitlab-org/gitlab-ce.git"),
            VcsUrl {
                provider: "gitlab.example.com".into(),
                id: "gitlab-org/gitlab-ce".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("https://gitlab.com/gitlab-org/gitlab-ce"),
            VcsUrl {
                provider: "gitlab.com".into(),
                id: "gitlab-org/gitlab-ce".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@gitlab.com:gitlab-org/gitlab-ce.git"),
            VcsUrl {
                provider: "gitlab.com".into(),
                id: "gitlab-org/gitlab-ce".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse(
                "https://source.developers.google.com/p/project-slug/r/github_org-slug_repo-slug"
            ),
            VcsUrl {
                provider: "source.developers.google.com".into(),
                id: "org-slug/repo-slug".into(),
            }
        );
        assert_eq!(
            VcsUrl::parse("git@gitlab.com:gitlab-org/GitLab-CE.git"),
            VcsUrl {
                provider: "gitlab.com".into(),
                id: "gitlab-org/gitlab-ce".into(),
            }
        );
    }

    #[test]
    fn test_get_repo_from_remote_preserve_case() {
        // Test that case-preserving function maintains original casing
        assert_eq!(
            get_repo_from_remote_preserve_case("https://github.com/MyOrg/MyRepo"),
            "MyOrg/MyRepo"
        );
        assert_eq!(
            get_repo_from_remote_preserve_case("git@github.com:SentryOrg/SentryRepo.git"),
            "SentryOrg/SentryRepo"
        );
        assert_eq!(
            get_repo_from_remote_preserve_case("https://gitlab.com/MyCompany/MyProject"),
            "MyCompany/MyProject"
        );
        assert_eq!(
            get_repo_from_remote_preserve_case("git@bitbucket.org:TeamName/ProjectName.git"),
            "TeamName/ProjectName"
        );
        assert_eq!(
            get_repo_from_remote_preserve_case("ssh://git@github.com/MyUser/MyRepo.git"),
            "MyUser/MyRepo"
        );

        // Test that regular function still lowercases
        assert_eq!(
            get_repo_from_remote("https://github.com/MyOrg/MyRepo"),
            "myorg/myrepo"
        );

        // Test edge cases - should fall back to lowercase when regex doesn't match
        assert_eq!(
            get_repo_from_remote_preserve_case("invalid-url"),
            get_repo_from_remote("invalid-url")
        );
    }

    #[test]
    fn test_extract_provider_name() {
        // Test basic provider name extraction
        assert_eq!(extract_provider_name("github.com"), "github");
        assert_eq!(extract_provider_name("gitlab.com"), "gitlab");
        assert_eq!(extract_provider_name("bitbucket.org"), "bitbucket");

        // Test edge case with trailing dots
        assert_eq!(extract_provider_name("github.com."), "github");

        // Test subdomain cases - we want the part before TLD, not the subdomain
        assert_eq!(extract_provider_name("api.github.com"), "github");
        assert_eq!(extract_provider_name("ssh.dev.azure.com"), "azure");
        assert_eq!(extract_provider_name("dev.azure.com"), "azure");

        // Test single component (no dots)
        assert_eq!(extract_provider_name("localhost"), "localhost");
        assert_eq!(extract_provider_name("myserver"), "myserver");

        // Test empty string
        assert_eq!(extract_provider_name(""), "");
    }

    #[test]
    fn test_get_provider_from_remote() {
        // Test that get_provider_from_remote normalizes provider names
        assert_eq!(
            get_provider_from_remote("https://github.com/user/repo"),
            "github"
        );
        assert_eq!(
            get_provider_from_remote("git@gitlab.com:user/repo.git"),
            "gitlab"
        );
        assert_eq!(
            get_provider_from_remote("https://bitbucket.org/user/repo"),
            "bitbucket"
        );
        assert_eq!(
            get_provider_from_remote("https://dev.azure.com/user/repo"),
            "azure"
        );
        assert_eq!(
            get_provider_from_remote("https://github.mycompany.com/user/repo"),
            "mycompany"
        );
        assert_eq!(
            get_provider_from_remote("https://source.developers.google.com/p/project/r/repo"),
            "google"
        );
        // Test edge case with trailing dot in hostname
        assert_eq!(
            get_provider_from_remote("https://github.com./user/repo"),
            "github"
        );
    }

    #[test]
    fn test_url_normalization() {
        assert!(!is_matching_url(
            "http://github.mycompany.com/mitsuhiko/flask",
            "git@github.com:mitsuhiko/flask.git"
        ));
        assert!(!is_matching_url(
            "git@github.mycompany.com/mitsuhiko/flask",
            "git@github.com:mitsuhiko/flask.git"
        ));
        assert!(is_matching_url(
            "http://github.com/mitsuhiko/flask",
            "git@github.com:mitsuhiko/flask.git"
        ));
        assert!(is_matching_url(
            "https://gitlab.com/gitlab-org/gitlab-ce",
            "git@gitlab.com:gitlab-org/gitlab-ce.git"
        ));
        assert!(is_matching_url(
            "https://gitlab.example.com/gitlab-org/gitlab-ce",
            "git@gitlab.example.com:gitlab-org/gitlab-ce.git"
        ));
        assert!(is_matching_url(
            "https://gitlab.example.com/gitlab-org/GitLab-CE",
            "git@gitlab.example.com:gitlab-org/gitlab-ce.git"
        ));
        assert!(is_matching_url(
            "https://gitlab.example.com/gitlab-org/GitLab-CE",
            "ssh://git@gitlab.example.com:22/gitlab-org/GitLab-CE"
        ));
        assert!(is_matching_url(
            "git@ssh.dev.azure.com:v3/project/repo/repo",
            "https://dev.azure.com/project/repo/_git/repo"
        ));
        assert!(is_matching_url(
            "git@ssh.dev.azure.com:v3/company/Repo%20Online/Repo%20Online",
            "https://dev.azure.com/company/Repo%20Online/_git/Repo%20Online"
        ));
        assert!(is_matching_url(
            "git://git@github.com/kamilogorek/picklerick.git",
            "https://github.com/kamilogorek/picklerick"
        ));
        assert!(is_matching_url(
            "git+ssh://git@github.com/kamilogorek/picklerick.git",
            "https://github.com/kamilogorek/picklerick"
        ));
        assert!(is_matching_url(
            "git+http://git@github.com/kamilogorek/picklerick.git",
            "https://github.com/kamilogorek/picklerick"
        ));
        assert!(is_matching_url(
            "git+https://git@github.com/kamilogorek/picklerick.git",
            "https://github.com/kamilogorek/picklerick"
        ));
    }

    fn git_initialize_repo() -> TempDir {
        let dir = tempdir().expect("Failed to generate temp dir.");

        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `git init`.")
            .wait()
            .expect("Failed to wait on `git init`.");

        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `git branch`.")
            .wait()
            .expect("Failed to wait on `git branch`.");

        Command::new("git")
            .args(["config", "--local", "user.name", "test"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `git config`.")
            .wait()
            .expect("Failed to wait on `git config`.");

        Command::new("git")
            .args(["config", "--local", "user.email", "test@example.com"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `git config`.")
            .wait()
            .expect("Failed to wait on `git config`.");

        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/getsentry/sentry-cli",
            ])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `git remote add`.")
            .wait()
            .expect("Failed to wait on `git remote add`.");

        Command::new("git")
            .args(["config", "--local", "commit.gpgsign", "false"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `config --local commit.gpgsign false`.")
            .wait()
            .expect("Failed to wait on `config --local commit.gpgsign false`.");

        Command::new("git")
            .args(["config", "--local", "tag.gpgsign", "false"])
            .current_dir(&dir)
            .spawn()
            .expect("Failed to execute `config --local tag.gpgsign false`.")
            .wait()
            .expect("Failed to wait on `config --local tag.gpgsign false`.");

        dir
    }

    fn git_create_commit(dir: &Path, file_path: &str, content: &[u8], commit_message: &str) {
        let path = dir.join(file_path);
        let mut file = File::create(path).expect("Failed to execute.");
        file.write_all(content).expect("Failed to execute.");

        let mut add = Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .spawn()
            .expect("Failed to execute `git add .`");

        add.wait().expect("Failed to wait on `git add`.");

        let mut commit = Command::new("git")
            .args([
                "commit",
                "-am",
                commit_message,
                "--author",
                "John Doe <john.doe@example.com>",
                "--quiet",
                "--no-edit",
            ])
            .current_dir(dir)
            .spawn()
            .expect("Failed to execute `git commit -m {message}`.");

        commit.wait().expect("Failed to wait on `git commit`.");
    }

    fn git_create_tag(dir: &Path, tag_name: &str, annotated: bool) -> String {
        let mut tag_cmd = vec!["tag", tag_name];

        if annotated {
            tag_cmd.push("-a");
            tag_cmd.push("-m");
            tag_cmd.push("imannotatedtag");
        }

        let mut tag = Command::new("git")
            .args(tag_cmd)
            .current_dir(dir)
            .spawn()
            .unwrap_or_else(|_| panic!("Failed to execute `git tag {tag_name}`"));

        tag.wait().expect("Failed to wait on `git tag`.");

        let hash = Command::new("git")
            .args(["rev-list", "-n", "1", tag_name])
            .current_dir(dir)
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute `git rev-list -n 1 {tag_name}`."));

        String::from_utf8(hash.stdout)
            .map(|s| s.trim().to_owned())
            .expect("Invalid utf-8")
    }

    #[test]
    fn test_get_commits_from_git() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 2\");",
            "\"second commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");
        let commits = get_commits_from_git(&repo, "", 20, false).expect("Failed");

        assert_debug_snapshot!(commits
            .0
            .iter()
            .map(|c| {
                (
                    c.author().name().unwrap().to_owned(),
                    c.author().email().unwrap().to_owned(),
                    c.summary(),
                )
            })
            .collect::<Vec<_>>());
    }

    #[test]
    fn test_generate_patch_set_base() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 2\");",
            "\"second commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 3\");",
            "\"third commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");
        let commits = get_commits_from_git(&repo, "", 20, false).expect("Failed");
        let patch_set =
            generate_patch_set(&repo, commits.0, commits.1, "example/test-repo").expect("Failed");

        assert_yaml_snapshot!(patch_set, {
            ".*.id" => "[id]",
            ".*.timestamp" => "[timestamp]"
        });
    }

    #[test]
    fn test_generate_patch_set_previous_commit() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 2\");",
            "\"second commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 3\");",
            "\"third commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");
        let head = repo.revparse_single("HEAD").expect("Failed");

        git_create_commit(
            dir.path(),
            "foo4.js",
            b"console.log(\"Hello, world! Part 4\");",
            "\"fourth commit\"",
        );

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world! Part 5\");",
            "\"fifth commit\"",
        );

        let commits =
            get_commits_from_git(&repo, &head.id().to_string(), 20, false).expect("Failed");
        let patch_set =
            generate_patch_set(&repo, commits.0, commits.1, "example/test-repo").expect("Failed");

        assert_yaml_snapshot!(patch_set, {
            ".*.id" => "[id]",
            ".*.timestamp" => "[timestamp]"
        });
    }

    #[test]
    fn test_generate_patch_default_twenty() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        for n in 0..20 {
            let file = format!("foo{n}.js");
            git_create_commit(
                dir.path(),
                &file,
                b"console.log(\"Hello, world! Part 2\");",
                "\"another commit\"",
            );
        }

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world!\");",
            "\"final commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");
        let commits = get_commits_from_git(&repo, "", 20, false).expect("Failed");
        let patch_set =
            generate_patch_set(&repo, commits.0, commits.1, "example/test-repo").expect("Failed");

        assert_yaml_snapshot!(patch_set, {
            ".*.id" => "[id]",
            ".*.timestamp" => "[timestamp]"
        });
    }

    #[test]
    fn test_generate_patch_ignore_missing() {
        let dir = git_initialize_repo();

        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        for n in 0..5 {
            let file = format!("foo{n}.js");
            git_create_commit(
                dir.path(),
                &file,
                b"console.log(\"Hello, world! Part 2\");",
                "\"another commit\"",
            );
        }

        git_create_commit(
            dir.path(),
            "foo2.js",
            b"console.log(\"Hello, world!\");",
            "\"final commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");
        let commits = get_commits_from_git(&repo, "nonexistinghash", 5, true).expect("Failed");
        let patch_set =
            generate_patch_set(&repo, commits.0, commits.1, "example/test-repo").expect("Failed");

        assert_yaml_snapshot!(patch_set, {
            ".*.id" => "[id]",
            ".*.timestamp" => "[timestamp]"
        });
    }

    #[test]
    fn test_git_repo_head_ref() {
        let dir = git_initialize_repo();

        // Create initial commit
        git_create_commit(
            dir.path(),
            "foo.js",
            b"console.log(\"Hello, world!\");",
            "\"initial commit\"",
        );

        let repo = git2::Repository::open(dir.path()).expect("Failed");

        // Test on a branch (should succeed)
        let head_ref = git_repo_head_ref(&repo).expect("Should get branch reference");
        assert_eq!(head_ref, "main");

        // Test in detached HEAD state (should fail)
        let head_commit = repo.head().unwrap().target().unwrap();
        repo.set_head_detached(head_commit)
            .expect("Failed to detach HEAD");

        let head_ref_result = git_repo_head_ref(&repo);
        assert!(head_ref_result.is_err());
        assert_eq!(
            head_ref_result.unwrap_err().to_string(),
            "HEAD is detached - no branch reference available"
        );
    }

    #[test]
    fn test_get_github_pr_number() {
        std::env::set_var("GITHUB_EVENT_NAME", "pull_request");
        std::env::set_var("GITHUB_REF", "refs/pull/123/merge");
        let pr_number = get_github_pr_number();
        assert_eq!(pr_number, Some(123));
        std::env::set_var("GITHUB_EVENT_NAME", "push");
        let pr_number = get_github_pr_number();
        assert_eq!(pr_number, None);
        std::env::set_var("GITHUB_EVENT_NAME", "pull_request");
        std::env::set_var("GITHUB_REF", "refs/heads/main");
        let pr_number = get_github_pr_number();
        assert_eq!(pr_number, None);
        std::env::remove_var("GITHUB_EVENT_NAME");
        std::env::remove_var("GITHUB_REF");
    }

    #[test]
    fn test_get_github_base_ref() {
        std::env::set_var("GITHUB_EVENT_NAME", "pull_request");
        std::env::set_var("GITHUB_BASE_REF", "main");
        let base_ref = get_github_base_ref();
        assert_eq!(base_ref, Some("main".to_owned()));

        // Test with different base branch
        std::env::set_var("GITHUB_BASE_REF", "develop");
        let base_ref = get_github_base_ref();
        assert_eq!(base_ref, Some("develop".to_owned()));

        // Test when not in pull_request event
        std::env::set_var("GITHUB_EVENT_NAME", "push");
        let base_ref = get_github_base_ref();
        assert_eq!(base_ref, None);

        // Test when GITHUB_BASE_REF is not set
        std::env::set_var("GITHUB_EVENT_NAME", "pull_request");
        std::env::remove_var("GITHUB_BASE_REF");
        let base_ref = get_github_base_ref();
        assert_eq!(base_ref, None);

        std::env::remove_var("GITHUB_EVENT_NAME");
    }

    #[test]
    fn test_extract_pr_head_sha_from_event() {
        let pr_json = r#"{
  "action": "opened",
  "number": 123,
  "pull_request": {
    "id": 789,
    "head": {
      "ref": "feature-branch",
      "sha": "19ef6adc4dbddf733db6e833e1f96fb056b6dba5"
    },
    "base": {
      "ref": "main",
      "sha": "55e6bc8c264ce95164314275d805f477650c440d"
    }
  }
}"#;

        assert_eq!(
            extract_pr_head_sha_from_event(pr_json),
            Some("19ef6adc4dbddf733db6e833e1f96fb056b6dba5".to_owned())
        );

        let push_json = r#"{
  "action": "push",
  "ref": "refs/heads/main",
  "head_commit": {
    "id": "xyz789abc123"
  }
}"#;

        assert_eq!(extract_pr_head_sha_from_event(push_json), None);
        let malformed_json = r#"{
  "pull_request": {
    "id": 789,
    "head": {
      "ref": "feature-branch"
    }
  }
}"#;

        assert_eq!(extract_pr_head_sha_from_event(malformed_json), None);

        assert_eq!(extract_pr_head_sha_from_event("{}"), None);
        let real_gh_json = r#"{
  "action": "synchronize",
  "pull_request": {
    "id": 2852219630,
    "head": {
      "label": "getsentry:no/test-pr-head-sha-workflow",
      "ref": "no/test-pr-head-sha-workflow",
      "sha": "19ef6adc4dbddf733db6e833e1f96fb056b6dba4"
    },
    "base": {
      "label": "getsentry:master",
      "ref": "master",
      "sha": "55e6bc8c264ce95164314275d805f477650c440d"
    }
  }
}"#;

        assert_eq!(
            extract_pr_head_sha_from_event(real_gh_json),
            Some("19ef6adc4dbddf733db6e833e1f96fb056b6dba4".to_owned())
        );
        let malicious_json = r#"{
  "action": "opened",
  "pull_request": {
    "title": "Fix \"pull_request\": {\"head\": {\"sha\": \"maliciousha123456789012345678901234567890\"}}",
    "body": "This PR contains \"head\": and \"sha\": patterns in the description",
    "head": {
      "ref": "feature-branch",
      "sha": "19ef6adc4dbddf733db6e833e1f96fb056b6dba5"
    }
  }
}"#;

        assert_eq!(
            extract_pr_head_sha_from_event(malicious_json),
            Some("19ef6adc4dbddf733db6e833e1f96fb056b6dba5".to_owned())
        );
        let any_sha_json = r#"{
  "pull_request": {
    "head": {
      "sha": "invalid-sha-123"
    }
  }
}"#;

        assert_eq!(
            extract_pr_head_sha_from_event(any_sha_json),
            Some("invalid-sha-123".to_owned())
        );

        assert_eq!(extract_pr_head_sha_from_event("invalid json {"), None);
    }

    #[test]
    fn test_find_head_with_github_event_path() {
        use std::fs;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let event_file = temp_dir.path().join("event.json");
        let pr_json = r#"{
  "action": "opened",
  "pull_request": {
    "head": {
      "sha": "19ef6adc4dbddf733db6e833e1f96fb056b6dba5"
    }
  }
}"#;

        fs::write(&event_file, pr_json).expect("Failed to write event file");

        std::env::set_var("GITHUB_EVENT_PATH", event_file.to_str().unwrap());
        let result = find_head();
        std::env::remove_var("GITHUB_EVENT_PATH");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "19ef6adc4dbddf733db6e833e1f96fb056b6dba5");
    }
}
