use std::borrow::Cow;

use anyhow::{Context as _, Result};
use clap::ArgMatches;
use log::{debug, info, warn};
use sha1_smol::Digest;

use crate::api::VcsInfo;
use crate::config::Config;
use crate::utils::vcs::{
    self, get_github_base_ref, get_github_head_ref, get_github_pr_number, get_provider_from_remote,
    get_repo_from_remote_preserve_case, git_repo_base_ref, git_repo_base_repo_name_preserve_case,
    git_repo_head_ref, git_repo_remote_url,
};

/// Collects git metadata from arguments and VCS introspection.
///
/// When `auto_collect` is false, only explicitly provided values are collected;
/// automatic inference from git repository and CI environment is skipped.
pub fn collect_git_metadata(
    matches: &ArgMatches,
    config: &Config,
    auto_collect: bool,
) -> VcsInfo<'static> {
    let head_sha = matches
        .get_one::<Option<Digest>>("head_sha")
        .map(|d| d.as_ref().cloned())
        .or_else(|| auto_collect.then(|| vcs::find_head_sha().ok()))
        .flatten();

    let cached_remote = config.get_cached_vcs_remote();
    let (vcs_provider, head_repo_name, head_ref, base_ref, base_repo_name) = {
        let repo = if auto_collect {
            git2::Repository::open_from_env().ok()
        } else {
            None
        };
        let repo_ref = repo.as_ref();
        let remote_url = repo_ref.and_then(|repo| git_repo_remote_url(repo, &cached_remote).ok());

        let vcs_provider = matches
            .get_one("vcs_provider")
            .cloned()
            .or_else(|| {
                auto_collect
                    .then(|| remote_url.as_ref().map(|url| get_provider_from_remote(url)))?
            })
            .unwrap_or_default();

        let head_repo_name = matches
            .get_one("head_repo_name")
            .cloned()
            .or_else(|| {
                auto_collect.then(|| {
                    remote_url
                        .as_ref()
                        .map(|url| get_repo_from_remote_preserve_case(url))
                })?
            })
            .unwrap_or_default();

        let head_ref = matches
            .get_one("head_ref")
            .cloned()
            .or_else(|| auto_collect.then(get_github_head_ref)?)
            .or_else(|| {
                auto_collect.then(|| {
                    repo_ref.and_then(|r| match git_repo_head_ref(r) {
                        Ok(ref_name) => {
                            debug!("Found current branch reference: {ref_name}");
                            Some(ref_name)
                        }
                        Err(e) => {
                            debug!("No valid branch reference found (likely detached HEAD): {e}");
                            None
                        }
                    })
                })?
            })
            .unwrap_or_default();

        let base_ref = matches
            .get_one("base_ref")
            .cloned()
            .or_else(|| auto_collect.then(get_github_base_ref)?)
            .or_else(|| {
                auto_collect.then(|| {
                    repo_ref.and_then(|r| match git_repo_base_ref(r, &cached_remote) {
                        Ok(base_ref_name) => {
                            debug!("Found base reference: {base_ref_name}");
                            Some(base_ref_name)
                        }
                        Err(e) => {
                            info!("Could not detect base branch reference: {e}");
                            None
                        }
                    })
                })?
            })
            .unwrap_or_default();

        let base_repo_name = matches
            .get_one("base_repo_name")
            .cloned()
            .or_else(|| {
                auto_collect.then(|| {
                    repo_ref.and_then(|r| match git_repo_base_repo_name_preserve_case(r) {
                        Ok(Some(base_repo_name)) => {
                            debug!("Found base repository name: {base_repo_name}");
                            Some(base_repo_name)
                        }
                        Ok(None) => {
                            debug!("No base repository found - not a fork");
                            None
                        }
                        Err(e) => {
                            warn!("Could not detect base repository name: {e}");
                            None
                        }
                    })
                })?
            })
            .unwrap_or_default();

        (
            vcs_provider,
            head_repo_name,
            head_ref,
            base_ref,
            base_repo_name,
        )
    };

    let base_sha_from_user = matches.get_one::<Option<Digest>>("base_sha").is_some();
    let base_ref_from_user = matches.get_one::<String>("base_ref").is_some();

    let mut base_sha = matches
        .get_one::<Option<Digest>>("base_sha")
        .map(|d| d.as_ref().cloned())
        .or_else(|| {
            if auto_collect {
                Some(
                    vcs::find_base_sha(&cached_remote)
                        .inspect_err(|e| debug!("Error finding base SHA: {e}"))
                        .ok()
                        .flatten(),
                )
            } else {
                None
            }
        })
        .flatten();

    let mut base_ref = base_ref;

    // If base_sha equals head_sha and both were auto-inferred, skip setting base_sha and base_ref
    if !base_sha_from_user
        && !base_ref_from_user
        && base_sha.is_some()
        && head_sha.is_some()
        && base_sha == head_sha
    {
        debug!(
            "Base SHA equals head SHA ({}), and both were auto-inferred. Skipping base_sha and base_ref, but keeping head_sha.",
            base_sha.expect("base_sha is Some at this point")
        );
        base_sha = None;
        base_ref = "".into();
    }

    let pr_number = matches.get_one("pr_number").copied().or_else(|| {
        if auto_collect {
            get_github_pr_number()
        } else {
            None
        }
    });

    VcsInfo {
        head_sha,
        base_sha,
        vcs_provider: Cow::Owned(vcs_provider),
        head_repo_name: Cow::Owned(head_repo_name),
        base_repo_name: Cow::Owned(base_repo_name),
        head_ref: Cow::Owned(head_ref),
        base_ref: Cow::Owned(base_ref),
        pr_number,
    }
}

/// Utility function to parse a SHA1 digest, allowing empty strings.
///
/// Empty strings result in Ok(None), otherwise we return the parsed digest
/// or an error if the SHA is invalid.
pub fn parse_sha_allow_empty(sha: &str) -> Result<Option<Digest>> {
    if sha.is_empty() {
        return Ok(None);
    }

    let digest = sha
        .parse()
        .with_context(|| format!("{sha} is not a valid SHA1 digest"))?;

    Ok(Some(digest))
}
