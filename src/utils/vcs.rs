use std::fmt;
use std::path::PathBuf;

use failure::{bail, format_err, Error};
use if_chain::if_chain;
use lazy_static::lazy_static;
use log::{debug, info};
use regex::Regex;

use crate::api::{Ref, Repo};

#[derive(Copy, Clone)]
pub enum GitReference<'a> {
    Commit(git2::Oid),
    Symbolic(&'a str),
}

impl<'a> fmt::Display for GitReference<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GitReference::Commit(ref c) => write!(f, "{}", c),
            GitReference::Symbolic(ref s) => write!(f, "{}", s),
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

fn parse_rev_range(rng: &str) -> (Option<String>, String) {
    if rng == "" {
        return (None, "HEAD".into());
    }
    let mut iter = rng.rsplitn(2, "..");
    let rev = iter.next().unwrap_or("HEAD");
    (iter.next().map(str::to_owned), rev.to_string())
}

impl CommitSpec {
    pub fn parse(s: &str) -> Result<CommitSpec, Error> {
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
                Regex::new(r"^(?:ssh|https?)://(?:[^@]+@)?([^/]+)/(.+)$").unwrap();
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
        }

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
        VcsUrl {
            provider: host.to_lowercase(),
            id: strip_git_suffix(path).to_lowercase(),
        }
    }
}

fn is_matching_url(a: &str, b: &str) -> bool {
    VcsUrl::parse(a) == VcsUrl::parse(b)
}

fn find_reference_url(repo: &str, repos: &[Repo]) -> Result<String, Error> {
    let mut found_non_git = false;
    for configured_repo in repos {
        if configured_repo.name != repo {
            continue;
        }

        match configured_repo.provider.id.as_str() {
            "git"
            | "github"
            | "bitbucket"
            | "visualstudio"
            | "integrations:github"
            | "integrations:github_enterprise"
            | "integrations:gitlab"
            | "integrations:bitbucket"
            | "integrations:vsts" => {
                if let Some(ref url) = configured_repo.url {
                    debug!("  Got reference URL for repo {}: {}", repo, url);
                    return Ok(url.clone());
                }
            }
            _ => {
                debug!("  unknown repository {} skipped", configured_repo);
                found_non_git = true;
            }
        }
    }

    if found_non_git {
        bail!("For non git repositories explicit revisions are required");
    } else {
        bail!("Could not find matching repository for {}", repo);
    }
}

fn find_matching_rev(
    reference: GitReference<'_>,
    spec: &CommitSpec,
    repos: &[Repo],
    disable_discovery: bool,
) -> Result<Option<String>, Error> {
    macro_rules! log_match {
        ($ex:expr) => {{
            let val = $ex;
            info!("  -> found matching revision {}", val);
            val
        }};
    }

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

    let reference_url = find_reference_url(&spec.repo, repos)?;
    debug!("  Looking for reference URL {}", &reference_url);

    // direct reference in root repository found.  If we are in discovery
    // mode we want to also check for matching URLs.
    if_chain! {
        if let Ok(remote) = repo.find_remote("origin");
        if let Some(url) = remote.url();
        then {
            if !discovery || is_matching_url(url, &reference_url) {
                debug!("  found match: {} == {}", url, &reference_url);
                let head = repo.revparse_single(r)?;
                return Ok(Some(log_match!(head.id().to_string())));
            } else {
                debug!("  not a match: {} != {}", url, &reference_url);
            }
        }
    }

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

    info!("  -> no matching revision found");
    Ok(None)
}

fn find_matching_revs(
    spec: &CommitSpec,
    repos: &[Repo],
    disable_discovery: bool,
) -> Result<(Option<String>, String), Error> {
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

    let rev = if let Some(rev) =
        find_matching_rev(spec.reference(), &spec, &repos[..], disable_discovery)?
    {
        rev
    } else {
        return Err(error(spec.reference(), &spec.repo));
    };

    let prev_rev = if let Some(rev) = spec.prev_reference() {
        if let Some(rv) = find_matching_rev(rev, &spec, &repos[..], disable_discovery)? {
            Some(rv)
        } else {
            return Err(error(rev, &spec.repo));
        }
    } else {
        None
    };

    Ok((prev_rev, rev))
}

pub fn find_head() -> Result<String, Error> {
    let repo = git2::Repository::open_from_env()?;
    let head = repo.revparse_single("HEAD")?;
    Ok(head.id().to_string())
}

/// Given commit specs and repos this returns a list of head commits
/// from it.
pub fn find_heads(specs: Option<Vec<CommitSpec>>, repos: &[Repo]) -> Result<Vec<Ref>, Error> {
    let mut rv = vec![];

    // if commit specs were explicitly provided find head commits with
    // limited amounts of magic.
    if let Some(specs) = specs {
        for spec in &specs {
            let (prev_rev, rev) = find_matching_revs(&spec, &repos[..], specs.len() == 1)?;
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
            if let Some(rev) = find_matching_rev(spec.reference(), &spec, &repos[..], false)? {
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
        VcsUrl::parse("git@gitlab.com:gitlab-org/GitLab-CE.git"),
        VcsUrl {
            provider: "gitlab.com".into(),
            id: "gitlab-org/gitlab-ce".into(),
        }
    )
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
    ))
}
