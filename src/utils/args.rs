use anyhow::{bail, Result};
use chrono::{DateTime, TimeZone as _, Utc};
use clap::{Arg, ArgAction, Command};

fn validate_org(v: &str) -> Result<String, String> {
    if v.contains('/') || v == "." || v == ".." || v.contains(' ') {
        Err(
            "Invalid value for organization. Use the URL slug or the ID and not the name!"
                .to_owned(),
        )
    } else {
        Ok(v.to_owned())
    }
}

pub fn validate_project(v: &str) -> Result<String, String> {
    if v.contains('/')
        || v == "."
        || v == ".."
        || v.contains(' ')
        || v.contains('\n')
        || v.contains('\t')
        || v.contains('\r')
    {
        Err("Invalid value for project. Use the URL slug or the ID and not the name!".to_owned())
    } else {
        Ok(v.to_owned())
    }
}

/// Validate a release string.
pub fn validate_release(v: &str) -> Result<()> {
    if v.trim() != v {
        anyhow::bail!(
            "Invalid release version. Releases must not contain leading or trailing spaces."
        );
    } else if v.is_empty()
        || v == "."
        || v == ".."
        || v.find(&['\n', '\t', '\x0b', '\x0c', '\t', '/'][..])
            .is_some()
    {
        anyhow::bail!(
            "Invalid release version. Slashes and certain whitespace characters are not permitted."
        );
    }

    Ok(())
}

fn parse_release(v: &str) -> Result<String> {
    validate_release(v).map(|_| v.to_owned())
}

pub fn validate_distribution(v: &str) -> Result<String, String> {
    if v.trim() != v {
        Err(
            "Invalid distribution name. Distribution must not contain leading or trailing spaces."
                .to_owned(),
        )
    } else if bytecount::num_chars(v.as_bytes()) > 64 {
        Err(
            "Invalid distribution name. Distribution name must not be longer than 64 characters."
                .to_owned(),
        )
    } else {
        Ok(v.to_owned())
    }
}

pub fn get_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(int) = value.parse::<i64>() {
        #[expect(clippy::unwrap_used, reason = "legacy code")]
        Ok(Utc.timestamp_opt(int, 0).single().unwrap())
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        Ok(dt.with_timezone(&Utc))
    } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        Ok(dt.with_timezone(&Utc))
    } else {
        bail!("Not in valid format. Unix timestamp or ISO 8601 date expected.");
    }
}

pub trait ArgExt: Sized {
    fn org_arg(self) -> Self;
    fn project_arg(self, multiple: bool) -> Self;
    fn release_arg(self) -> Self;
    fn version_arg(self, global: bool) -> Self;
    fn git_metadata_args(self) -> Self;
}

impl ArgExt for Command {
    fn org_arg(self) -> Command {
        self.arg(
            Arg::new("org")
                .value_name("ORG")
                .long("org")
                .short('o')
                .value_parser(validate_org)
                .global(true)
                .help("The organization ID or slug."),
        )
    }

    fn project_arg(self, multiple: bool) -> Command {
        self.arg(
            Arg::new("project")
                .value_name("PROJECT")
                .long("project")
                .short('p')
                .value_parser(validate_project)
                .global(true)
                .action(if multiple {
                    ArgAction::Append
                } else {
                    ArgAction::Set
                })
                .help("The project ID or slug."),
        )
    }

    fn release_arg(self) -> Command {
        self.arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .short('r')
                .global(true)
                .allow_hyphen_values(true)
                .value_parser(parse_release)
                .help("The release slug."),
        )
    }

    fn version_arg(self, global: bool) -> Command {
        self.arg(
            Arg::new("version")
                .value_name("VERSION")
                // either specified for subcommands (global=true) or for this command (required=true)
                .required(!global)
                .global(global)
                .value_parser(parse_release)
                .help("The version of the release"),
        )
    }

    fn git_metadata_args(self) -> Command {
        use crate::utils::build_vcs::parse_sha_allow_empty;

        self.arg(
                Arg::new("head_sha")
                    .long("head-sha")
                    .value_parser(parse_sha_allow_empty)
                    .help("The VCS commit sha to use for the upload. If not provided, the current commit sha will be used.")
            )
            .arg(
                Arg::new("base_sha")
                    .long("base-sha")
                    .value_parser(parse_sha_allow_empty)
                    .help("The VCS commit's base sha to use for the upload. If not provided, the merge-base of the current and remote branch will be used.")
            )
            .arg(
                Arg::new("vcs_provider")
                    .long("vcs-provider")
                    .help("The VCS provider to use for the upload. If not provided, the current provider will be used.")
            )
            .arg(
                Arg::new("head_repo_name")
                    .long("head-repo-name")
                    .help("The name of the git repository to use for the upload (e.g. organization/repository). If not provided, the current repository will be used.")
            )
            .arg(
                Arg::new("base_repo_name")
                    .long("base-repo-name")
                    .help("The name of the git repository to use for the upload (e.g. organization/repository). If not provided, the current repository will be used.")
            )
            .arg(
                Arg::new("head_ref")
                    .long("head-ref")
                    .help("The reference (branch) to use for the upload. If not provided, the current reference will be used.")
            )
            .arg(
                Arg::new("base_ref")
                    .long("base-ref")
                    .help("The base reference (branch) to use for the upload. If not provided, the merge-base with the remote tracking branch will be used.")
            )
            .arg(
                Arg::new("pr_number")
                    .long("pr-number")
                    .value_parser(clap::value_parser!(u32))
                    .help("The pull request number to use for the upload. If not provided and running \
                        in a pull_request-triggered GitHub Actions workflow, the PR number will be automatically \
                        detected from GitHub Actions environment variables.")
            )
            .arg(
                Arg::new("force_git_metadata")
                    .long("force-git-metadata")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("no_git_metadata")
                    .help("Force collection and sending of git metadata (branch, commit, etc.). \
                        If neither this nor --no-git-metadata is specified, git metadata is \
                        automatically collected when running in most CI environments.")
            )
            .arg(
                Arg::new("no_git_metadata")
                    .long("no-git-metadata")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("force_git_metadata")
                    .help("Disable collection and sending of git metadata.")
            )
    }
}
