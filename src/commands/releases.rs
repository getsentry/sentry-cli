//! Implements a command for managing releases.
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::{bail, err_msg, Error};
use indicatif::HumanBytes;
use lazy_static::lazy_static;
use log::{debug, warn};
use regex::Regex;
use symbolic::debuginfo::sourcebundle::SourceFileType;

use crate::api::{
    Api, Deploy, FileContents, NewRelease, NoneReleaseInfo, OptionalReleaseInfo, ProgressBarMode,
    ReleaseStatus, UpdatedRelease,
};
use crate::config::Config;
use crate::utils::args::{
    get_timestamp, validate_int, validate_project, validate_timestamp, ArgExt,
};
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::{ReleaseFile, ReleaseFileUpload, UploadContext};
use crate::utils::formatting::{HumanDuration, Table};
use crate::utils::releases::detect_release_name;
use crate::utils::sourcemaps::SourceMapProcessor;
use crate::utils::system::QuietExit;
use crate::utils::vcs::{
    find_heads, generate_patch_set, get_commits_from_git, get_repo_from_remote, CommitSpec,
};

struct ReleaseContext<'a> {
    pub api: Arc<Api>,
    pub org: String,
    pub project_default: Option<&'a str>,
}

impl<'a> ReleaseContext<'a> {
    pub fn get_org(&'a self) -> Result<&str, Error> {
        Ok(&self.org)
    }

    pub fn get_project_default(&'a self) -> Result<String, Error> {
        if let Some(proj) = self.project_default {
            Ok(proj.to_string())
        } else {
            let config = Config::current();
            Ok(config.get_project_default()?)
        }
    }

    pub fn get_projects(&'a self, matches: &ArgMatches<'a>) -> Result<Vec<String>, Error> {
        if let Some(projects) = matches.values_of("projects") {
            Ok(projects.map(str::to_owned).collect())
        } else if let Some(project) = self.project_default {
            Ok(vec![project.to_string()])
        } else {
            let config = Config::current();
            Ok(vec![config.get_project_default()?])
        }
    }
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Manage releases on Sentry.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_arg()
        .arg(Arg::with_name("project")
            .hidden(true)
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .validator(validate_project))
        .subcommand(App::new("new")
            .about("Create a new release.")
            .version_arg(1)
            .projects_arg()
            // this is deprecated and no longer does anything
            .arg(Arg::with_name("ref")
                .long("ref")
                .value_name("REF")
                .hidden(true))
            .arg(Arg::with_name("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."))
            .arg(Arg::with_name("finalize")
                 .long("finalize")
                 .help("Immediately finalize the release. (sets it to released)")))
        .subcommand(App::new("propose-version")
            .about("Propose a version name for a new release."))
        .subcommand(App::new("set-commits")
            .about("Set commits of a release.")
            .version_arg(1)
            .arg(Arg::with_name("clear")
                 .long("clear")
                 .help("Clear all current commits from the release."))
            .arg(Arg::with_name("auto")
                .long("auto")
                .help("Enable completely automated commit management.{n}\
                        This requires that the command is run from within a git repository.  \
                        sentry-cli will then automatically find remotely configured \
                        repositories and discover commits."))
            .arg(Arg::with_name("ignore-missing")
                .long("ignore-missing")
                .help("When the flag is set and the previous release commit was not found in the repository, \
                        will create a release with the default commits count (or the one specified with `--initial-depth`) \
                        instead of failing the command."))
            .arg(Arg::with_name("ignore-empty")
                .long("ignore-empty")
                .help("When the flag is set, command will not fail and just exit silently \
                        if no new commits for a given release have been found."))
            .arg(Arg::with_name("local")
                .conflicts_with_all(&["auto", "clear", "commits", ])
                .long("local")
                .help("Set commits of a release from local git.{n}\
                        This requires that the command is run from within a git repository.  \
                        sentry-cli will then automatically find remotely configured \
                        repositories and discover commits."))
            .arg(Arg::with_name("initial-depth")
                .conflicts_with("auto")
                .long("initial-depth")
                .value_name("INITIAL DEPTH")
                .validator(validate_int)
                .help("Set the number of commits of the initial release. The default is 20."))
            .arg(Arg::with_name("commits")
                 .long("commit")
                 .short("c")
                 .value_name("SPEC")
                 .multiple(true)
                 .number_of_values(1)
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
                        which will force the revision to a certain value.")))
        .subcommand(App::new("delete")
            .about("Delete a release.")
            .version_arg(1))
        .subcommand(App::new("archive")
            .about("Archive a release.")
            .version_arg(1))
        .subcommand(App::new("restore")
            .about("Restore a release.")
            .version_arg(1))
        .subcommand(App::new("finalize")
            .about("Mark a release as finalized and released.")
            .version_arg(1)
            .arg(Arg::with_name("started")
                 .long("started")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("Set the release start date."))
            .arg(Arg::with_name("released")
                 .long("released")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("Set the release time. [defaults to the current time]")))
        .subcommand(App::new("list")
            .about("List the most recent releases.")
            .arg(Arg::with_name("no_abbrev")
                .long("no-abbrev")
                .hidden(true))
            .arg(Arg::with_name("show_projects")
                .short("P")
                .long("show-projects")
                .help("Display the Projects column"))
            .arg(Arg::with_name("raw")
                .short("R")
                .long("raw")
                .help("Print raw, delimiter separated list of releases. [defaults to new line]"))
            .arg(Arg::with_name("delimiter")
                .short("D")
                .long("delimiter")
                .takes_value(true)
                .requires("raw")
                .help("Delimiter for the --raw flag")))
        .subcommand(App::new("info")
            .about("Print information about a release.")
            .version_arg(1)
            .arg(Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Do not print any output.{n}If this is passed the command can be \
                       used to determine if a release already exists.  The exit status \
                       will be 0 if the release exists or 1 otherwise.")))
        .subcommand(App::new("files")
            .about("Manage release artifacts.")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .version_arg(1)
            .subcommand(App::new("list").about("List all release files."))
            .subcommand(App::new("delete")
                .about("Delete a release file.")
                .arg(Arg::with_name("all")
                    .short("A")
                    .long("all")
                    .help("Delete all files."))
                .arg(Arg::with_name("names")
                    .value_name("NAMES")
                    .index(1)
                    .multiple(true)
                    .help("Filenames to delete.")))
            .subcommand(App::new("upload")
                .about("Upload files for a release.")
                .arg(Arg::with_name("dist")
                    .long("dist")
                    .short("d")
                    .value_name("DISTRIBUTION")
                    .help("Optional distribution identifier for this file."))
                .arg(Arg::with_name("wait")
                    .long("wait")
                    .help("Wait for the server to fully process uploaded files."))
                .arg(Arg::with_name("headers")
                    .long("header")
                    .short("H")
                    .value_name("KEY VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Store a header with this file."))
                .arg(Arg::with_name("path")
                    .value_name("PATH")
                    .index(1)
                    .required(true)
                    .help("The path to the file or directory to upload."))
                .arg(Arg::with_name("name")
                    .index(2)
                    .value_name("NAME")
                    .help("The name of the file on the server."))
                .arg(Arg::with_name("url_prefix")
                    .short("u")
                    .long("url-prefix")
                    .value_name("PREFIX")
                    .help("The URL prefix to prepend to all filenames."))
                .arg(Arg::with_name("url_suffix")
                    .long("url-suffix")
                    .value_name("SUFFIX")
                    .help("The URL suffix to append to all filenames."))
                .arg(Arg::with_name("ignore")
                    .long("ignore")
                    .short("i")
                    .value_name("IGNORE")
                    .multiple(true)
                    .help("Ignores all files and folders matching the given glob"))
                .arg(Arg::with_name("ignore_file")
                    .long("ignore-file")
                    .short("I")
                    .value_name("IGNORE_FILE")
                    .help("Ignore all files and folders specified in the given \
                           ignore file, e.g. .gitignore."))
                .arg(Arg::with_name("extensions")
                    .long("ext")
                    .short("x")
                    .value_name("EXT")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Set the file extensions that are considered for upload. \
                           This overrides the default extensions. To add an extension, all default \
                           extensions must be repeated. Specify once per extension.")))
            .subcommand(App::new("upload-sourcemaps")
                .about("Upload sourcemaps for a release.")
                .arg(Arg::with_name("paths")
                    .value_name("PATHS")
                    .index(1)
                    .required_unless_one(&["bundle", "bundle_sourcemap"])
                    .multiple(true)
                    .help("The files to upload."))
                .arg(Arg::with_name("url_prefix")
                    .short("u")
                    .long("url-prefix")
                    .value_name("PREFIX")
                    .help("The URL prefix to prepend to all filenames."))
                .arg(Arg::with_name("url_suffix")
                    .long("url-suffix")
                    .value_name("SUFFIX")
                    .help("The URL suffix to append to all filenames."))
                .arg(Arg::with_name("dist")
                    .long("dist")
                    .short("d")
                    .value_name("DISTRIBUTION")
                    .help("Optional distribution identifier for the sourcemaps."))
                .arg(Arg::with_name("validate")
                    .long("validate")
                    .help("Enable basic sourcemap validation."))
                .arg(Arg::with_name("wait")
                    .long("wait")
                    .help("Wait for the server to fully process uploaded files."))
                .arg(Arg::with_name("no_sourcemap_reference")
                    .long("no-sourcemap-reference")
                    .help("Disable emitting of automatic sourcemap references.{n}\
                           By default the tool will store a 'Sourcemap' header with \
                           minified files so that sourcemaps are located automatically \
                           if the tool can detect a link. If this causes issues it can \
                           be disabled."))
                .arg(Arg::with_name("no_rewrite")
                    .long("no-rewrite")
                    .help("Disables rewriting of matching sourcemaps. By default the tool \
                        will rewrite sources, so that indexed maps are flattened and missing \
                        sources are inlined if possible.{n}This fundamentally \
                        changes the upload process to be based on sourcemaps \
                        and minified files exclusively and comes in handy for \
                        setups like react-native that generate sourcemaps that \
                        would otherwise not work for sentry.")
                    .conflicts_with("rewrite"))
                .arg(Arg::with_name("strip_prefix")
                    .long("strip-prefix")
                    .value_name("PREFIX")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Strips the given prefix from all sources references inside the upload \
                           sourcemaps (paths used within the sourcemap content, to map minified code \
                           to it's original source). Only sources that start with the given prefix \
                           will be stripped.{n}This will not modify the uploaded sources paths. \
                           To do that, point the upload or upload-sourcemaps command \
                           to a more precise directory instead.")
                    .conflicts_with("no_rewrite"))
                .arg(Arg::with_name("strip_common_prefix")
                    .long("strip-common-prefix")
                    .help("Similar to --strip-prefix but strips the most common \
                           prefix on all sources references.")
                    .conflicts_with("no_rewrite"))
                .arg(Arg::with_name("ignore")
                    .long("ignore")
                    .short("i")
                    .value_name("IGNORE")
                    .multiple(true)
                    .help("Ignores all files and folders matching the given glob"))
                .arg(Arg::with_name("ignore_file")
                    .long("ignore-file")
                    .short("I")
                    .value_name("IGNORE_FILE")
                    .help("Ignore all files and folders specified in the given \
                           ignore file, e.g. .gitignore."))
                .arg(Arg::with_name("bundle")
                    .long("bundle")
                    .value_name("BUNDLE")
                    .conflicts_with("paths")
                    .requires_all(&["bundle_sourcemap"])
                    .help("Path to the application bundle (indexed, file, or regular)"))
                .arg(Arg::with_name("bundle_sourcemap")
                    .long("bundle-sourcemap")
                    .value_name("BUNDLE_SOURCEMAP")
                    .conflicts_with("paths")
                    .requires_all(&["bundle"])
                    .help("Path to the bundle sourcemap"))
                // legacy parameter
                .arg(Arg::with_name("verbose")
                    .long("verbose")
                    .short("v")
                    .hidden(true))
                .arg(Arg::with_name("extensions")
                    .long("ext")
                    .short("x")
                    .value_name("EXT")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Set the file extensions that are considered for upload. \
                           This overrides the default extensions. To add an extension, all default \
                           extensions must be repeated. Specify once per extension.{n}\
                           Defaults to: `--ext=js --ext=map --ext=jsbundle --ext=bundle`"))
                .arg(Arg::with_name("rewrite")
                    .long("rewrite")
                    .help("Enables rewriting of matching sourcemaps \
                        so that indexed maps are flattened and missing \
                        sources are inlined if possible.{n}This fundamentally \
                        changes the upload process to be based on sourcemaps \
                        and minified files exclusively and comes in handy for \
                        setups like react-native that generate sourcemaps that \
                        would otherwise not work for sentry.")
                    .conflicts_with("no_rewrite")
                    .hidden(true))))
        .subcommand(App::new("deploys")
            .about("Manage release deployments.")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .version_arg(1)
            .subcommand(App::new("new")
                .about("Creates a new release deployment.")
                .arg(Arg::with_name("env")
                     .long("env")
                     .short("e")
                     .value_name("ENV")
                     .required(true)
                     .help("Set the environment for this release.{n}This argument is required.  \
                            Values that make sense here would be 'production' or 'staging'."))
                .arg(Arg::with_name("name")
                     .long("name")
                     .short("n")
                     .value_name("NAME")
                     .help("Optional human readable name for this deployment."))
                .arg(Arg::with_name("url")
                     .long("url")
                     .short("u")
                     .value_name("URL")
                     .help("Optional URL that points to the deployment."))
                .arg(Arg::with_name("started")
                     .long("started")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deployment started."))
                .arg(Arg::with_name("finished")
                     .long("finished")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deployment finished."))
                .arg(Arg::with_name("time")
                     .long("time")
                     .short("t")
                     .value_name("SECONDS")
                     .validator(validate_int)
                     .help("Optional deployment duration in seconds.{n}\
                            This can be specified alternatively to `--started` and `--finished`.")))
            .subcommand(App::new("list")
                .about("List all deployments of a release.")))
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

#[cfg(windows)]
fn path_as_url(path: &Path) -> String {
    path.display().to_string().replace("\\", "/")
}

#[cfg(not(windows))]
fn path_as_url(path: &Path) -> String {
    path.display().to_string()
}

fn execute_new<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let info_rv = ctx.api.new_release(
        ctx.get_org()?,
        &NewRelease {
            version: matches.value_of("version").unwrap().to_owned(),
            projects: ctx.get_projects(matches)?,
            url: matches.value_of("url").map(str::to_owned),
            date_started: Some(Utc::now()),
            date_released: if matches.is_present("finalize") {
                Some(Utc::now())
            } else {
                None
            },
        },
    )?;
    println!("Created release {}.", info_rv.version);
    Ok(())
}

fn execute_finalize<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    fn get_date(value: Option<&str>, now_default: bool) -> Result<Option<DateTime<Utc>>, Error> {
        match value {
            None => Ok(if now_default { Some(Utc::now()) } else { None }),
            Some(value) => Ok(Some(get_timestamp(value)?)),
        }
    }

    let info_rv = ctx.api.update_release(
        ctx.get_org()?,
        matches.value_of("version").unwrap(),
        &UpdatedRelease {
            projects: ctx.get_projects(matches).ok(),
            url: matches.value_of("url").map(str::to_owned),
            date_started: get_date(matches.value_of("started"), false)?,
            date_released: get_date(matches.value_of("released"), true)?,
            ..Default::default()
        },
    )?;
    println!("Finalized release {}.", info_rv.version);
    Ok(())
}

fn execute_propose_version() -> Result<(), Error> {
    println!("{}", detect_release_name()?);
    Ok(())
}

fn execute_set_commits<'a>(
    ctx: &ReleaseContext<'_>,
    matches: &ArgMatches<'a>,
) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();

    let org = ctx.get_org()?;
    let repos = ctx.api.list_organization_repos(org)?;
    let mut commit_specs = vec![];
    let config = Config::current();

    let heads = if repos.is_empty() {
        None
    } else if matches.is_present("auto") {
        let commits = find_heads(None, &repos, Some(config.get_cached_vcs_remote()))?;
        if commits.is_empty() {
            None
        } else {
            Some(commits)
        }
    } else if matches.is_present("clear") {
        Some(vec![])
    } else if matches.is_present("local") {
        None
    } else {
        if let Some(commits) = matches.values_of("commits") {
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
    if let Ok(projects) = ctx.get_projects(matches) {
        ctx.api.new_release(
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
        ctx.api.set_release_refs(&org, version, heads)?;
    } else {
        let default_count = matches
            .value_of("initial-depth")
            .unwrap_or("20")
            .parse::<usize>()?;

        if matches.is_present("auto") {
            println!("Could not determine any commits to be associated with a repo-based integration. Proceeding to find commits from local git tree.");
        }
        // Get the commit of the most recent release.
        let prev_commit = match ctx.api.get_previous_release_with_commits(org, version)? {
            OptionalReleaseInfo::Some(prev) => prev.last_commit.map(|c| c.id).unwrap_or_default(),
            OptionalReleaseInfo::None(NoneReleaseInfo {}) => String::new(),
        };

        // Find and connect to local git.
        let repo = git2::Repository::open_from_env()?;

        // Parse the git url.
        let remote = config.get_cached_vcs_remote();
        let parsed = get_repo_from_remote(&remote);
        let ignore_missing = matches.is_present("ignore-missing");
        // Fetch all the commits upto the `prev_commit` or return the default (20).
        // Will return a tuple of Vec<GitCommits> and the `prev_commit` if it exists in the git tree.
        let (commit_log, prev_commit) =
            get_commits_from_git(&repo, &prev_commit, default_count, ignore_missing)?;

        // Calculate the diff for each commit in the Vec<GitCommit>.
        let commits = generate_patch_set(&repo, commit_log, prev_commit, &parsed)?;

        if commits.is_empty() {
            // TODO(v2): Make it a default behavior on next major release instead?
            let ignore_empty = matches.is_present("ignore-empty");
            if ignore_empty {
                println!("No commits found. Leaving release alone.");
                return Ok(());
            } else {
                bail!("No commits found. Change commits range, initial depth or use --ignore-empty to allow empty patch sets.");
            }
        }

        ctx.api.update_release(
            ctx.get_org()?,
            version,
            &UpdatedRelease {
                commits: Some(commits),
                ..Default::default()
            },
        )?;

        println!("Success! Set commits for release {}.", version);
    }

    Ok(())
}

fn execute_delete<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let project = ctx.get_project_default().ok();
    if ctx
        .api
        .delete_release(ctx.get_org()?, project.as_deref(), version)?
    {
        println!("Deleted release {}!", version);
    } else {
        println!(
            "Did nothing. Release with this version ({}) does not exist.",
            version
        );
    }
    Ok(())
}

fn execute_archive<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let info_rv = ctx.api.update_release(
        ctx.get_org()?,
        version,
        &UpdatedRelease {
            projects: Some(vec![]),
            version: Some(version.into()),
            status: Some(ReleaseStatus::Archived),
            ..Default::default()
        },
    )?;
    println!("Archived release {}.", info_rv.version);
    Ok(())
}

fn execute_restore<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let info_rv = ctx.api.update_release(
        ctx.get_org()?,
        version,
        &UpdatedRelease {
            projects: Some(vec![]),
            version: Some(version.into()),
            status: Some(ReleaseStatus::Open),
            ..Default::default()
        },
    )?;
    println!("Restored release {}.", info_rv.version);
    Ok(())
}

fn execute_list<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let project = ctx.get_project_default().ok();
    let releases = ctx.api.list_releases(ctx.get_org()?, project.as_deref())?;

    if matches.is_present("raw") {
        let versions = releases
            .iter()
            .map(|release_info| release_info.version.clone())
            .collect::<Vec<_>>()
            .join(matches.value_of("delimiter").unwrap_or("\n"));

        println!("{}", versions);
        return Ok(());
    }

    let mut table = Table::new();
    let title_row = table.title_row();
    title_row.add("Released").add("Version");
    if matches.is_present("show_projects") {
        title_row.add("Projects");
    }
    title_row.add("New Events").add("Last Event");
    for release_info in releases {
        let row = table.add_row();
        if let Some(date) = release_info.date_released {
            row.add(format!(
                "{} ago",
                HumanDuration(Utc::now().signed_duration_since(date))
            ));
        } else {
            row.add("(unreleased)");
        }
        row.add(&release_info.version);
        if matches.is_present("show_projects") {
            let project_slugs = release_info
                .projects
                .into_iter()
                .map(|p| p.slug)
                .collect::<Vec<_>>();
            if !project_slugs.is_empty() {
                row.add(project_slugs.join("\n"));
            } else {
                row.add("-");
            }
        }
        row.add(release_info.new_groups);
        if let Some(date) = release_info.last_event {
            row.add(format!(
                "{} ago",
                HumanDuration(Utc::now().signed_duration_since(date))
            ));
        } else {
            row.add("-");
        }
    }
    table.print();
    Ok(())
}

fn execute_info<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    let release = ctx.api.get_release(org, project.as_deref(), &version)?;

    // quiet mode just exists
    if matches.is_present("quiet") {
        if release.is_none() {
            return Err(QuietExit(1).into());
        }
        return Ok(());
    }

    if let Some(release) = release {
        let mut tbl = Table::new();
        tbl.add_row().add("Version").add(&release.version);
        tbl.add_row().add("Date created").add(&release.date_created);
        if let Some(last_event) = release.last_event {
            tbl.add_row().add("Last event").add(last_event);
        }
        tbl.print();
    } else {
        println!("No such release");
        return Err(QuietExit(1).into());
    }
    Ok(())
}

fn execute_files_list<'a>(
    ctx: &ReleaseContext<'_>,
    _matches: &ArgMatches<'a>,
    release: &str,
) -> Result<(), Error> {
    let mut table = Table::new();
    table
        .title_row()
        .add("Name")
        .add("Distribution")
        .add("Source Map")
        .add("Size");

    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    for artifact in ctx
        .api
        .list_release_files(org, project.as_deref(), release)?
    {
        let row = table.add_row();
        row.add(&artifact.name);
        if let Some(ref dist) = artifact.dist {
            row.add(dist);
        } else {
            row.add("");
        }
        if let Some(sm_ref) = artifact.get_sourcemap_reference() {
            row.add(sm_ref);
        } else {
            row.add("");
        }
        row.add(HumanBytes(artifact.size));
    }

    table.print();

    Ok(())
}

fn execute_files_delete<'a>(
    ctx: &ReleaseContext<'_>,
    matches: &ArgMatches<'a>,
    release: &str,
) -> Result<(), Error> {
    let files: HashSet<String> = match matches.values_of("names") {
        Some(paths) => paths.map(|x| x.into()).collect(),
        None => HashSet::new(),
    };
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    for file in ctx
        .api
        .list_release_files(org, project.as_deref(), release)?
    {
        if !(matches.is_present("all") || files.contains(&file.name)) {
            continue;
        }
        if ctx
            .api
            .delete_release_file(org, project.as_deref(), release, &file.id)?
        {
            println!("D {}", file.name);
        }
    }
    Ok(())
}

fn execute_files_upload<'a>(
    ctx: &ReleaseContext<'_>,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let dist = matches.value_of("dist");
    let mut headers = vec![];
    if let Some(header_list) = matches.values_of("header") {
        for header in header_list {
            if !header.contains(':') {
                bail!("Invalid header. Needs to be in key:value format");
            }
            let mut iter = header.splitn(2, ':');
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    };
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    let path = Path::new(matches.value_of("path").unwrap());

    // Batch files upload
    if path.is_dir() {
        let ignore_file = matches.value_of("ignore_file").unwrap_or("");
        let ignores = matches
            .values_of("ignore")
            .map(|ignores| ignores.map(|i| format!("!{}", i)).collect())
            .unwrap_or_else(Vec::new);
        let extensions = matches
            .values_of("extensions")
            .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
            .unwrap_or_else(Vec::new);

        let sources = ReleaseFileSearch::new(path.to_path_buf())
            .ignore_file(ignore_file)
            .ignores(ignores)
            .extensions(extensions)
            .collect_files()?;

        let url_prefix = get_url_prefix_from_args(matches);
        let url_suffix = get_url_suffix_from_args(matches);
        let files = sources
            .iter()
            .map(|source| {
                let local_path = source.path.strip_prefix(&source.base_path).unwrap();
                let url = format!("{}/{}{}", url_prefix, path_as_url(local_path), url_suffix);

                (
                    url.to_string(),
                    ReleaseFile {
                        url,
                        path: source.path.clone(),
                        contents: source.contents.clone(),
                        ty: SourceFileType::Source,
                        headers: headers.clone(),
                        messages: vec![],
                    },
                )
            })
            .collect();

        let ctx = &UploadContext {
            org,
            project: project.as_deref(),
            release: &version,
            dist,
            wait: matches.is_present("wait"),
        };

        ReleaseFileUpload::new(ctx).files(&files).upload()
    }
    // Single file upload
    else {
        let name = match matches.value_of("name") {
            Some(name) => name,
            None => Path::new(path)
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| err_msg("No filename provided."))?,
        };

        if let Some(artifact) = ctx.api.upload_release_file(
            org,
            project.as_deref(),
            &version,
            &FileContents::FromPath(path),
            &name,
            dist,
            Some(&headers[..]),
            ProgressBarMode::Request,
        )? {
            println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
        } else {
            bail!("File already present!");
        }
        Ok(())
    }
}

fn get_url_prefix_from_args<'a, 'b>(matches: &'b ArgMatches<'a>) -> &'b str {
    let mut rv = matches.value_of("url_prefix").unwrap_or("~");
    // remove a single slash from the end.  so ~/ becomes ~ and app:/// becomes app://
    if rv.ends_with('/') {
        rv = &rv[..rv.len() - 1];
    }
    rv
}

fn get_url_suffix_from_args<'a, 'b>(matches: &'b ArgMatches<'a>) -> &'b str {
    matches.value_of("url_suffix").unwrap_or("")
}

fn get_prefixes_from_args<'a, 'b>(matches: &'b ArgMatches<'a>) -> Vec<&'b str> {
    let mut prefixes: Vec<&str> = match matches.values_of("strip_prefix") {
        Some(paths) => paths.collect(),
        None => vec![],
    };
    if matches.is_present("strip_common_prefix") {
        prefixes.push("~");
    }
    prefixes
}

fn process_sources_from_bundle<'a>(
    matches: &ArgMatches<'a>,
    processor: &mut SourceMapProcessor,
) -> Result<(), Error> {
    let url_prefix = get_url_prefix_from_args(matches);
    let url_suffix = get_url_suffix_from_args(matches);

    let bundle_path = PathBuf::from(matches.value_of("bundle").unwrap());
    let bundle_url = format!(
        "{}/{}{}",
        url_prefix,
        bundle_path.file_name().unwrap().to_string_lossy(),
        url_suffix
    );

    let sourcemap_path = PathBuf::from(matches.value_of("bundle_sourcemap").unwrap());
    let sourcemap_url = format!(
        "{}/{}{}",
        url_prefix,
        sourcemap_path.file_name().unwrap().to_string_lossy(),
        url_suffix
    );

    debug!("Bundle path: {}", bundle_path.display());
    debug!("Sourcemap path: {}", sourcemap_path.display());

    processor.add(
        &bundle_url,
        ReleaseFileSearch::collect_file(bundle_path.clone())?,
    )?;
    processor.add(
        &sourcemap_url,
        ReleaseFileSearch::collect_file(sourcemap_path)?,
    )?;

    if let Ok(ram_bundle) = sourcemap::ram_bundle::RamBundle::parse_unbundle_from_path(&bundle_path)
    {
        debug!("File RAM bundle found, extracting its contents...");
        // For file ("unbundle") RAM bundles we need to explicitly unpack it, otherwise we cannot detect it
        // reliably inside "processor.rewrite()"
        processor.unpack_ram_bundle(&ram_bundle, &bundle_url)?;
    } else if sourcemap::ram_bundle::RamBundle::parse_indexed_from_path(&bundle_path).is_ok() {
        debug!("Indexed RAM bundle found");
    } else {
        warn!("Regular bundle found");
    }

    let mut prefixes = get_prefixes_from_args(matches);
    if !prefixes.contains(&"~") {
        prefixes.push("~");
    }
    debug!("Prefixes: {:?}", prefixes);

    processor.rewrite(&prefixes)?;
    processor.add_sourcemap_references()?;

    Ok(())
}

fn process_sources_from_paths<'a>(
    matches: &ArgMatches<'a>,
    processor: &mut SourceMapProcessor,
) -> Result<(), Error> {
    let paths = matches.values_of("paths").unwrap();
    let ignore_file = matches.value_of("ignore_file").unwrap_or("");
    let extensions = matches
        .values_of("extensions")
        .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
        .unwrap_or_else(|| vec!["js", "map", "jsbundle", "bundle"]);
    let ignores = matches
        .values_of("ignore")
        .map(|ignores| ignores.map(|i| format!("!{}", i)).collect())
        .unwrap_or_else(Vec::new);

    for path in paths {
        // if we start walking over something that is an actual file then
        // the directory iterator yields that path and terminates.  We
        // handle that case here specifically to figure out what the path is
        // we should strip off.
        let path = Path::new(path);
        let (base_path, check_ignore) = if path.is_file() {
            (path.parent().unwrap(), false)
        } else {
            (path, true)
        };

        let mut search = ReleaseFileSearch::new(path.to_path_buf());

        if check_ignore {
            search
                .ignore_file(ignore_file)
                .ignores(ignores.clone())
                .extensions(extensions.clone());
        }

        let sources = search.collect_files()?;

        let url_prefix = get_url_prefix_from_args(matches);
        let url_suffix = get_url_suffix_from_args(matches);

        for source in sources {
            let local_path = source.path.strip_prefix(base_path).unwrap();
            let url = format!("{}/{}{}", url_prefix, path_as_url(local_path), url_suffix);
            processor.add(&url, source)?;
        }
    }

    if !matches.is_present("no_rewrite") {
        let prefixes = get_prefixes_from_args(matches);
        processor.rewrite(&prefixes)?;
    }

    if !matches.is_present("no_sourcemap_reference") {
        processor.add_sourcemap_references()?;
    }

    if matches.is_present("validate") {
        processor.validate_all()?;
    }

    Ok(())
}

fn execute_files_upload_sourcemaps<'a>(
    ctx: &ReleaseContext<'_>,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let mut processor = SourceMapProcessor::new();

    if matches.is_present("bundle") && matches.is_present("bundle_sourcemap") {
        process_sources_from_bundle(matches, &mut processor)?;
    } else {
        process_sources_from_paths(matches, &mut processor)?;
    }

    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();

    // make sure the release exists
    let release = ctx.api.new_release(
        &org,
        &NewRelease {
            version: version.into(),
            projects: ctx.get_projects(matches)?,
            ..Default::default()
        },
    )?;

    processor.upload(&UploadContext {
        org,
        project: project.as_deref(),
        release: &release.version,
        dist: matches.value_of("dist"),
        wait: matches.is_present("wait"),
    })?;

    Ok(())
}

fn execute_files<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let release = matches.value_of("version").unwrap();
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_files_list(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        return execute_files_delete(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("upload") {
        return execute_files_upload(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("upload-sourcemaps") {
        return execute_files_upload_sourcemaps(ctx, sub_matches, release);
    }
    unreachable!();
}

fn execute_deploys_new<'a>(
    ctx: &ReleaseContext<'_>,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let mut deploy = Deploy {
        env: matches.value_of("env").unwrap().to_string(),
        name: matches.value_of("name").map(str::to_owned),
        url: matches.value_of("url").map(str::to_owned),
        ..Default::default()
    };

    if let Some(value) = matches.value_of("time") {
        let finished = Utc::now();
        deploy.finished = Some(finished);
        deploy.started = Some(finished - Duration::seconds(value.parse().unwrap()));
    } else {
        if let Some(finished_str) = matches.value_of("finished") {
            deploy.finished = Some(get_timestamp(finished_str)?);
        } else {
            deploy.finished = Some(Utc::now());
        }
        if let Some(started_str) = matches.value_of("started") {
            deploy.started = Some(get_timestamp(started_str)?);
        }
    }

    let org = ctx.get_org()?;
    let deploy = ctx.api.create_deploy(org, version, &deploy)?;

    println!("Created new deploy {} for '{}'", deploy.name(), deploy.env);

    Ok(())
}

fn execute_deploys_list<'a>(
    ctx: &ReleaseContext<'_>,
    _matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let mut table = Table::new();
    table
        .title_row()
        .add("Environment")
        .add("Name")
        .add("Finished");

    for deploy in ctx.api.list_deploys(ctx.get_org()?, version)? {
        table
            .add_row()
            .add(&deploy.env)
            .add(deploy.name())
            .add(HumanDuration(
                Utc::now().signed_duration_since(deploy.finished.unwrap()),
            ));
    }

    if table.is_empty() {
        println!("No deploys found");
    } else {
        table.print();
    }

    Ok(())
}

fn execute_deploys<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let release = matches.value_of("version").unwrap();
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_deploys_new(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_deploys_list(ctx, sub_matches, release);
    }
    unreachable!();
}

pub fn execute(matches: &ArgMatches<'_>) -> Result<(), Error> {
    // this one does not need a context or org
    if let Some(_sub_matches) = matches.subcommand_matches("propose-version") {
        return execute_propose_version();
    }

    let config = Config::current();
    let ctx = ReleaseContext {
        api: Api::current(),
        org: config.get_org(matches)?,
        project_default: matches.value_of("project"),
    };
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_new(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("finalize") {
        return execute_finalize(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("set-commits") {
        return execute_set_commits(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        return execute_delete(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("archive") {
        return execute_archive(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("restore") {
        return execute_restore(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_list(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("info") {
        return execute_info(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("files") {
        return execute_files(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("deploys") {
        return execute_deploys(&ctx, sub_matches);
    }
    unreachable!();
}
