use std::fmt;
use std::path::PathBuf;

use git2;
use regex::Regex;

use prelude::*;
use api::{Repo, Ref};


pub enum GitReference<'a> {
    Commit(git2::Oid),
    Symbolic(&'a str),
}

impl<'a> fmt::Display for GitReference<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    pub rev: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VcsUrl {
    pub provider: &'static str,
    pub id: String,
}

impl CommitSpec {
    pub fn parse(s: &str) -> Result<CommitSpec> {
        lazy_static! {
            static ref SPEC_RE: Regex = Regex::new(
                r"^([^@#]+)(?:#([^@]+))?(?:@(.+))?$").unwrap();
        }
        if let Some(caps) = SPEC_RE.captures(s) {
            Ok(CommitSpec {
                repo: caps[1].to_string(),
                path: caps.get(2).map(|x| PathBuf::from(x.as_str())),
                rev: caps.get(3).map(|x| x.as_str().to_string()),
            })
        } else {
            Err(Error::from(format!("Could not parse commit spec '{}'", s)))
        }
    }

    pub fn reference<'a>(&'a self) -> GitReference<'a> {
        let r = self.rev.as_ref().map(|x| x.as_str()).unwrap_or("HEAD");
        if let Ok(oid) = git2::Oid::from_str(r) {
            GitReference::Commit(oid)
        } else {
            GitReference::Symbolic(r)
        }
    }
}

fn strip_git_suffix(s: &str) -> &str {
    if s.ends_with(".git") {
        &s[0..s.len() - 4]
    } else {
        s
    }
}

impl VcsUrl {
    pub fn parse(url: &str) -> VcsUrl {
        lazy_static! {
            static ref GITHUB_URL_RE: Regex = Regex::new(
                r"^(?:ssh|https?)://(?:[^@]+@)?github.com/([^/]+)/([^/]+)").unwrap();
            static ref GITHUB_SSH_RE: Regex = Regex::new(
                r"^(?:[^@]+@)?github.com:([^/]+)/([^/]+)").unwrap();
        }
        if let Some(caps) = GITHUB_URL_RE.captures(url) {
            VcsUrl {
                provider: "github",
                id: format!("{}/{}", &caps[1], strip_git_suffix(&caps[2])),
            }
        } else if let Some(caps) = GITHUB_SSH_RE.captures(url) {
            VcsUrl {
                provider: "github",
                id: format!("{}/{}", &caps[1], strip_git_suffix(&caps[2])),
            }
        } else {
            VcsUrl {
                provider: "generic",
                id: url.into(),
            }
        }
    }
}

fn is_matching_url(a: &str, b: &str) -> bool {
    VcsUrl::parse(a) == VcsUrl::parse(b)
}

fn find_reference_url(repo: &str, repos: &[Repo]) -> Result<String> {
    for configured_repo in repos {
        if configured_repo.name != repo {
            continue;
        }
        if &configured_repo.provider.id != "github" &&
           &configured_repo.provider.id != "git" {
            return Err(Error::from("For non git repositories \
                                   explicit revisions are required"));
        }

        return Ok(configured_repo.url.clone())
    }

    Err(Error::from(format!("Could not find matching repository for {}", repo)))
}

fn find_matching_rev(spec: &CommitSpec, repos: &[Repo], disable_discovery: bool)
    -> Result<Option<String>>
{
    let r = match spec.reference() {
        GitReference::Commit(commit) => {
            return Ok(Some(commit.to_string()));
        }
        GitReference::Symbolic(r) => r,
    };

    let (repo, discovery) = if let Some(ref path) = spec.path {
        (git2::Repository::open(path)?, false)
    } else {
        (git2::Repository::open_from_env()?, !disable_discovery)
    };

    let reference_url = find_reference_url(&spec.repo, repos)?;

    // direct reference in root repository found.  If we are in discovery
    // mode we want to also check for matching URLs.
    if_chain! {
        if let Ok(remote) = repo.find_remote("origin");
        if let Some(url) = remote.url();
        if !discovery || is_matching_url(url, &reference_url);
        then {
            let head = repo.revparse_single(r)?;
            return Ok(Some(head.id().to_string()));
        }
    }

    // in discovery mode we want to find that repo in associated submodules.
    for submodule in repo.submodules()? {
        if_chain! {
            if let Some(submodule_url) = submodule.url();
            if is_matching_url(submodule_url, &reference_url);
            then {
                // heads on submodules is easier so let's start with that
                // because that does not require the submodule to be
                // checked out.
                if_chain! {
                    if r == "HEAD";
                    if let Some(head_oid) = submodule.head_id();
                    then {
                        return Ok(Some(head_oid.to_string()));
                    }
                }

                // otherwise we need to open the submodule which requires
                // it to be checked out.
                if_chain! {
                    if let Ok(subrepo) = submodule.open();
                    then {
                        let head = subrepo.revparse_single(r)?;
                        return Ok(Some(head.id().to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

pub fn find_head() -> Result<String> {
    let repo = git2::Repository::open_from_env()?;
    let head = repo.revparse_single("HEAD")?;
    Ok(head.id().to_string())
}

/// Given commit specs and repos this returns a list of head commits
/// from it.
pub fn find_heads(specs: Option<Vec<CommitSpec>>, repos: Vec<Repo>)
    -> Result<Vec<Ref>>
{
    let mut rv = vec![];

    // if commit specs were explicitly provided find head commits with
    // limited amounts of magic.
    if let Some(specs) = specs {
        for spec in &specs {
            let rev = if let Some(rev) = find_matching_rev(
                &spec, &repos[..], specs.len() == 1)? {
                rev
            } else {
                return Err(Error::from(format!(
                    "Could not find commit '{}' for '{}'. If you do not have local \
                     checkouts of the repositories in question referencing tags or \
                     other references will not work and you need to refer to \
                     revisions explicitly.",
                    spec.reference(), &spec.repo)));
            };
            rv.push(Ref {
                repo: spec.repo.clone(),
                rev: rev,
            });
        }

    // otherwise apply all the magic available
    } else {
        for repo in &repos {
            if let Some(head) = find_matching_rev(
                &CommitSpec {
                    repo: repo.name.to_string(),
                    path: None,
                    rev: Some("HEAD".into()),
                }, &repos[..], false)? {
                rv.push(Ref {
                    repo: repo.name.to_string(),
                    rev: head,
                });
            }
        }
    }

    Ok(rv)
}

#[test]
fn test_url_parsing() {
    assert_eq!(VcsUrl::parse("http://github.com/mitsuhiko/flask"), VcsUrl {
        provider: "github",
        id: "mitsuhiko/flask".into(),
    });
    assert_eq!(VcsUrl::parse("git@github.com:mitsuhiko/flask.git"), VcsUrl {
        provider: "github",
        id: "mitsuhiko/flask".into(),
    });
}

#[test]
fn test_url_normalization() {
    assert!(is_matching_url("http://github.com/mitsuhiko/flask",
                            "git@github.com:mitsuhiko/flask.git"));
}
