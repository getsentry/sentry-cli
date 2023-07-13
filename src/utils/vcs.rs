use std::fmt;
use std::path::PathBuf;

use anyhow::{bail, format_err, Error, Result};
use chrono::{DateTime, FixedOffset, TimeZone};
use git2::{Commit, Repository, Time};
use if_chain::if_chain;
use lazy_static::lazy_static;
use log::{debug, info};
use regex::Regex;

use crate::api::{GitCommit, PatchSet, Ref, Repo};

#[derive(Copy, Clone)]
pub enum GitReference<'a> {
    Commit(git2::Oid),
    Symbolic(&'a str),
}

impl<'a> fmt::Display for GitReference<'a> {
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
    (iter.next().map(str::to_owned), rev.to_string())
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
                    id: format!("{}/{}", username.to_lowercase(), &caps[1].to_lowercase()),
                };
            }
            if let Some(caps) = VS_TRAILING_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: host.to_lowercase(),
                    id: caps[1].to_lowercase(),
                };
            }
        }

        if let Some(caps) = AZUREDEV_DOMAIN_RE.captures(host) {
            let hostname = &caps[1];
            if let Some(caps) = AZUREDEV_VERSION_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: hostname.into(),
                    id: format!("{}/{}", &caps[1].to_lowercase(), &caps[2].to_lowercase()),
                };
            }
            if let Some(caps) = VS_TRAILING_GIT_PATH_RE.captures(path) {
                return VcsUrl {
                    provider: hostname.to_lowercase(),
                    id: caps[1].to_lowercase(),
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
                id: format!("{}/{}", &caps[1].to_lowercase(), &caps[2].to_lowercase()),
            };
        }

        VcsUrl {
            provider: host.to_lowercase(),
            id: strip_git_suffix(path).to_lowercase(),
        }
    }
}

fn is_matching_url(a: &str, b: &str) -> bool {
    VcsUrl::parse(a) == VcsUrl::parse(b)
}

pub fn get_repo_from_remote(repo: &str) -> String {
    let obj = VcsUrl::parse(repo);
    obj.id
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
                if let Ok(remote) = repo.find_remote(&remote_name.unwrap_or_else(|| "origin".to_string()));
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
    let repo = git2::Repository::open_from_env()?;
    let head = repo.revparse_single("HEAD")?;
    Ok(head.id().to_string())
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
                repo: repo.name.to_string(),
                path: None,
                rev: "HEAD".into(),
                prev_rev: None,
            };
            if let Some(rev) =
                find_matching_rev(spec.reference(), &spec, repos, false, remote_name.clone())?
            {
                rv.push(Ref {
                    repo: repo.name.to_string(),
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
) -> Result<(Vec<Commit>, Option<Commit>)> {
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
            repository: repository.to_string(),
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
use {
    crate::api::RepoProvider,
    insta::{assert_debug_snapshot, assert_yaml_snapshot},
    std::fs::File,
    std::io::Write,
    std::path::Path,
    std::process::Command,
    tempfile::tempdir,
};

#[test]
fn test_find_matching_rev_with_lightweight_tag() {
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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

#[cfg(test)]
fn git_initialize_repo(dir: &Path) {
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git init`.")
        .wait()
        .expect("Failed to wait on git init.");

    Command::new("git")
        .args(["config", "--local", "user.name", "test"])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git config`.")
        .wait()
        .expect("Failed to wait on git config.");

    Command::new("git")
        .args(["config", "--local", "user.email", "test@example.com"])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git config`.")
        .wait()
        .expect("Failed to wait on git config.");

    Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/getsentry/sentry-cli",
        ])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git remote add`.")
        .wait()
        .expect("Failed to wait on git remote add.");
}

#[cfg(test)]
fn git_create_commit(dir: &Path, file_path: &str, content: &[u8], commit_message: &str) {
    let path = dir.join(file_path);
    let mut file = File::create(path).expect("Failed to execute.");
    file.write_all(content).expect("Failed to execute.");

    let mut add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git add .`");

    add.wait().expect("Failed to wait on git add.");

    let mut commit = Command::new("git")
        .args([
            "commit",
            "-am",
            commit_message,
            "--author",
            "John Doe <john.doe@example.com>",
            "--quiet",
        ])
        .current_dir(dir)
        .spawn()
        .expect("Failed to execute `git commit -m {message}`.");

    commit.wait().expect("Failed to wait on git commit.");
}

#[cfg(test)]
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

    tag.wait().expect("Failed to wait on git tag.");

    let hash = Command::new("git")
        .args(["rev-list", "-n", "1", tag_name])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute `git rev-list -n 1 {tag_name}`."));

    String::from_utf8(hash.stdout)
        .map(|s| s.trim().to_string())
        .expect("Invalid utf-8")
}

#[test]
fn test_get_commits_from_git() {
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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

    let commits = get_commits_from_git(&repo, &head.id().to_string(), 20, false).expect("Failed");
    let patch_set =
        generate_patch_set(&repo, commits.0, commits.1, "example/test-repo").expect("Failed");

    assert_yaml_snapshot!(patch_set, {
        ".*.id" => "[id]",
        ".*.timestamp" => "[timestamp]"
    });
}

#[test]
fn test_generate_patch_default_twenty() {
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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
    let dir = tempdir().expect("Failed to generate temp dir.");
    git_initialize_repo(dir.path());

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
