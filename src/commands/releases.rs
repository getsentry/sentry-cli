//! Implements a command for managing releases.
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use chrono::{DateTime, Duration, TimeZone, Utc};
use clap::{App, AppSettings, ArgMatches};
use failure::{err_msg, Error};
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use indicatif::HumanBytes;
use regex::Regex;

use api::{Api, Deploy, FileContents, NewRelease, UpdatedRelease};
use config::Config;
use utils::args::{validate_org, validate_project};
use utils::formatting::{HumanDuration, Table};
use utils::releases::detect_release_name;
use utils::sourcemaps::SourceMapProcessor;
use utils::system::QuietExit;
use utils::vcs::{find_heads, CommitSpec};

struct ReleaseContext<'a> {
    pub api: Rc<Api>,
    pub org: String,
    pub project_default: Option<&'a str>,
}

impl<'a> ReleaseContext<'a> {
    pub fn get_org(&'a self) -> Result<&str, Error> {
        Ok(&self.org)
    }

    pub fn get_project_default(&'a self) -> Result<String, Error> {
        if let Some(ref proj) = self.project_default {
            Ok(proj.to_string())
        } else {
            let config = Config::get_current();
            Ok(config.get_project_default()?)
        }
    }

    pub fn get_projects(&'a self, matches: &ArgMatches<'a>) -> Result<Vec<String>, Error> {
        if let Some(projects) = matches.values_of("projects") {
            Ok(projects.map(|x| x.to_string()).collect())
        } else if let Some(project) = self.project_default {
            Ok(vec![project.to_string()])
        } else {
            let config = Config::get_current();
            Ok(vec![config.get_project_default()?])
        }
    }
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    clap_app!(@app (app)
        (about: "Manage releases on Sentry.")
        (setting: AppSettings::SubcommandRequiredElseHelp)
        (@arg org: -o --org [ORGANIZATION] {validate_org} "The organization slug.")
        (@arg project: -p --project [PROJECT] {validate_project} +hidden)
        (@subcommand new =>
            (about: "Create a new release.")
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@arg projects: -p --project [PROJECT]... {validate_project} "Project slugs.")
            (@arg ref: --ref [REF] +hidden)
            (@arg url: --url [URL] "Optional URL to the release for information purposes.")
            (@arg finalize: --finalize "Immediately finalize the release (sets it to released).")
        )
        (@subcommand propose_version => (about: "Propose a version name for a new release."))
        (@subcommand set_commits =>
            (about: "Set commits of a release.")
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@arg clear: --clear "Clear all current commits from the release.")
            (@arg auto: --auto
                "Enable completely automated commit management.{n}\
                 This requires that the command is run from within a git repository.  \
                 sentry-cli will then automatically find remotely configured \
                 repositories and discover commits.")
            (@arg commits: -c --commit [SPEC]...
                "Defines a single commit for a repo as \
                 identified by the repo name in the remote Sentry config. \
                 If no commit has been specified sentry-cli will attempt \
                 to auto discover that repository in the local git repo \
                 and then use the HEAD commit.  This will either use the \
                 current git repository or attempt to auto discover a \
                 submodule with a compatible URL.{n}{n}\
                 The value can be provided as `REPO` in which case sentry-cli \
                 will auto-discover the commit based on reachable repositories. \
                 Alternatively it can be provided as `REPO#PATH` in which case \
                 the current commit of the repository at the given PATH is \
                 assumed.  To override the revision `@REV` can be appended \
                 which will force the revision to a certain value.")
        )
        (@subcommand delete =>
            (about: "Delete a release.")
            (@arg version: <VERSION> {validate_version} "The version of the release.")
        )
        (@subcommand finalize =>
            (about: "Mark a release as finalized and released.")
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@arg started: --started [TIMESTAMP] {validate_timestamp} "Set the release start date.")
            (@arg released: --released [TIMESTAMP] {validate_timestamp}
                "Set the release time. [defaults to the current time]")
        )
        (@subcommand list =>
            (about: "List the most recent release.")
            (@arg no_abbrev: --("no-abbrev") "Do not abbreviate the release version.")
        )
        (@subcommand info =>
            (about: "Print information about a release."))
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@arg quiet: -q --quiet
                "Do not print any output.{n}If this is passed the command can be \
                 used to determine if a release already exists.  The exit status \
                 will be 0 if the release exists or 1 otherwise.")
        (@subcommand files =>
            (about: "Manage release artifacts.")
            (setting: AppSettings::SubcommandRequiredElseHelp)
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@subcommand list => (about: "List all release files."))
            (@subcommand delete =>
                (about: "Delete a release file.")
                (@arg all: -A --all "Delete all files")
                (@arg names: [NAMES]... "Filenames to delete.")
            )
            (@subcommand upload =>
                (about: "Upload a file for a release.")
                (@arg dist: -d --dist [DISTRIBUTION] "Optional distribution identifier.")
                (@arg headers: -H --header [KEY_VALUE]... "Store a header with this file.")
                (@arg path: <PATH> "The path to the file to upload.")
                (@arg name: [NAME] "The name of the file on the server.")
            )
            (@subcommand upload_sourcemap =>
                (about: "Upload sourcemaps for a release.")
                (@arg paths: <PATH>... "The files to upload")
                (@arg url_prefix: -u --("url-prefix") [PREFIX] "The URL prefix to prepend to all filenames.")
                (@arg url_suffix: --("url-suffix") [SUFFIX] "The URL suffix to append to all filenames.")
                (@arg dist: -d --dist [DISTRIBUTION] "Optional distribution identifier for the sourcemaps.")
                (@arg validate: --validate "Enable basic sourcemap validation.")
                (@arg no_sourcemap_reference: --("no-sourcemap-reference")
                    "Disable emitting of automatic sourcemap references.{n}\
                     By default the tool will store a 'Sourcemap' header with \
                     minified files so that sourcemaps are located automatically \
                     if the tool can detect a link. If this causes issues it can \
                     be disabled.")
                (@arg no_rewrite: --("no-rewrite")
                    "Disables rewriting of matching sourcemaps \
                     so that indexed maps are flattened and missing \
                     sources are inlined if possible.{n}This fundamentally \
                     changes the upload process to be based on sourcemaps \
                     and minified files exclusively and comes in handy for \
                     setups like react-native that generate sourcemaps that \
                     would otherwise not work for sentry.")
                (@arg rewrite: --rewrite
                    "Enables rewriting of matching sourcemaps \
                     so that indexed maps are flattened and missing \
                     sources are inlined if possible.{n}This fundamentally \
                     changes the upload process to be based on sourcemaps \
                     and minified files exclusively and comes in handy for \
                     setups like react-native that generate sourcemaps that \
                     would otherwise not work for sentry.")
                (@arg strip_prefix: --("strip-prefix") [PREFIX]...
                    "Strip the given prefix from all filenames.{n}\
                     Only files that start with the given prefix will be stripped.")
                (@arg strip_common_prefix: --("strip-common-prefix")
                    "Similar to --strip-prefix but strips the most common \
                     prefix on all sources.")
                (@arg ignore: -i --ignore [GLOB]...
                    "Ignores all files and folders matching the given glob.")
                (@arg ignore_file: -I --("ignore-file") [GLOB]...
                    "Ignore all files and folders specified in the given \
                     ignore file, e.g. .gitignore.")
                (@arg extensions: -x --ext [EXT]...
                    "Add a file extension to the list of files to upload.")
                (@arg verbose: -v --verbose +hidden)
            )
        )
        (@subcommand deploys =>
            (about: "Manage release deployments.")
            (setting: AppSettings::SubcommandRequiredElseHelp)
            (@arg version: <VERSION> {validate_version} "The version of the release.")
            (@subcommand new =>
                (about: "Creates a new release deployment.")
                (@arg env: -e --env <ENV>
                    "Set the environment for this release.{n}This argument is required.  \
                     Values that make sense here would be 'production' or 'staging'.")
                (@arg name: -n --name [NAME] "Optional human readable name for this deployment.")
                (@arg url: -u --url [URL] "Optional URL that points to the deployment.")
                (@arg started: --started [TIMESTAMP] {validate_timestamp}
                    "Optional unix timestamp when the deployment started.")
                (@arg finished: --finished [TIMESTAMP] {validate_timestamp}
                    "Optional unix timestamp when the deployment finished.")
                (@arg time: -t --time [SECONDS] {validate_seconds}
                    "Optional deployment duration in seconds.{n}\
                     This can be specified alternatively to `--started` and `--finished`.")
            )
            (@subcommand list =>
                (about: "List all deployments of a release.")
            )
        )
    )
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
fn validate_version(v: String) -> Result<(), String> {
    if v.trim() != v {
        return Err(
            "Invalid release version. Releases must not contain leading or trailing spaces."
                .to_string(),
        );
    }

    if v.is_empty() || v == "." || v == ".." || v
        .find(&['\n', '\t', '\x0b', '\x0c', '\t', '/'][..])
        .is_some()
    {
        return Err(
            "Invalid release version. Slashes and certain whitespace characters are not permitted."
                .to_string(),
        );
    }

    Ok(())
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
fn validate_seconds(v: String) -> Result<(), String> {
    if v.parse::<i64>().is_ok() {
        Ok(())
    } else {
        Err("Invalid value (seconds as integer required)".to_string())
    }
}

fn get_timestamp(value: &str) -> Result<DateTime<Utc>, Error> {
    if let Ok(int) = value.parse::<i64>() {
        Ok(Utc.timestamp(int, 0))
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        Ok(dt.with_timezone(&Utc))
    } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        Ok(dt.with_timezone(&Utc))
    } else {
        bail!("not in valid format. Unix timestamp or ISO 8601 date expected.");
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
fn validate_timestamp(v: String) -> Result<(), String> {
    if let Err(err) = get_timestamp(&v) {
        Err(err.to_string())
    } else {
        Ok(())
    }
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

fn strip_version(version: &str) -> &str {
    lazy_static! {
        static ref DOTTED_PATH_PREFIX_RE: Regex =
            Regex::new(r"^([a-z][a-z0-9-]+)(\.[a-z][a-z0-9-]+)+-").unwrap();
    }
    if let Some(m) = DOTTED_PATH_PREFIX_RE.find(version) {
        strip_sha(&version[m.end()..])
    } else {
        strip_sha(version)
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

fn execute_new<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let info_rv = ctx.api.new_release(
        ctx.get_org()?,
        &NewRelease {
            version: matches.value_of("version").unwrap().to_owned(),
            projects: ctx.get_projects(matches)?,
            url: matches.value_of("url").map(|x| x.to_owned()),
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

fn execute_finalize<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
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
            url: matches.value_of("url").map(|x| x.to_owned()),
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

fn execute_set_commits<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let org = ctx.get_org()?;
    let repos = ctx.api.list_organization_repos(org)?;
    let mut commit_specs = vec![];

    if repos.is_empty() {
        bail!(
            "No repositories are configured in Sentry for \
             your organization."
        );
    }

    let heads = if matches.is_present("auto") {
        let commits = find_heads(None, &repos)?;
        if commits.is_empty() {
            bail!(
                "Could not determine any commits to be associated \
                 automatically. You will have to explicitly provide \
                 commits on the command line."
            );
        }
        Some(commits)
    } else if matches.is_present("clear") {
        Some(vec![])
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
        let commits = find_heads(Some(commit_specs), &repos)?;
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
        println!("No commits found. Leaving release alone.");
    }

    Ok(())
}

fn execute_delete<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let project = ctx.get_project_default().ok();
    if ctx.api.delete_release(
        ctx.get_org()?,
        project.as_ref().map(|x| x.as_str()),
        version,
    )? {
        println!("Deleted release {}!", version);
    } else {
        println!(
            "Did nothing. Release with this version ({}) does not exist.",
            version
        );
    }
    Ok(())
}

fn execute_list<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let project = ctx.get_project_default().ok();
    let releases = ctx
        .api
        .list_releases(ctx.get_org()?, project.as_ref().map(|x| x.as_str()))?;
    let abbrev = !matches.is_present("no_abbrev");
    let mut table = Table::new();
    table
        .title_row()
        .add("Released")
        .add("Version")
        .add("New Events")
        .add("Last Event");
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
        if abbrev {
            row.add(strip_version(&release_info.version));
        } else {
            row.add(&release_info.version);
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

fn execute_info<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    let release = ctx
        .api
        .get_release(org, project.as_ref().map(|x| x.as_str()), &version)?;

    // quiet mode just exists
    if matches.is_present("quiet") {
        if release.is_none() {
            return Err(QuietExit(1).into());
        }
        return Ok(());
    }

    if let Some(release) = release {
        let short_version = strip_version(&release.version);
        let mut tbl = Table::new();
        tbl.add_row().add("Version").add(short_version);
        if short_version != release.version {
            tbl.add_row().add("Full version").add(&release.version);
        }
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
    ctx: &ReleaseContext,
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
    for artifact in
        ctx.api
            .list_release_files(org, project.as_ref().map(|x| x.as_str()), release)?
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
    ctx: &ReleaseContext,
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
        .list_release_files(org, project.as_ref().map(|x| x.as_str()), release)?
    {
        if !(matches.is_present("all") || files.contains(&file.name)) {
            continue;
        }
        if ctx.api.delete_release_file(
            org,
            project.as_ref().map(|x| x.as_str()),
            release,
            &file.id,
        )? {
            println!("D {}", file.name);
        }
    }
    Ok(())
}

fn execute_files_upload<'a>(
    ctx: &ReleaseContext,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let path = Path::new(matches.value_of("path").unwrap());
    let name = match matches.value_of("name") {
        Some(name) => name,
        None => Path::new(path)
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| err_msg("No filename provided."))?,
    };
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
    if let Some(artifact) = ctx.api.upload_release_file(
        org,
        project.as_ref().map(|x| x.as_str()),
        &version,
        &FileContents::FromPath(&path),
        &name,
        dist,
        Some(&headers[..]),
    )? {
        println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
    } else {
        bail!("File already present!");
    }
    Ok(())
}

fn execute_files_upload_sourcemaps<'a>(
    ctx: &ReleaseContext,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let url_prefix = matches
        .value_of("url_prefix")
        .unwrap_or("~")
        .trim_right_matches('/');
    let url_suffix = matches.value_of("url_suffix").unwrap_or("");
    let paths = matches.values_of("paths").unwrap();
    let extensions = matches
        .values_of("extensions")
        .map(|extensions| extensions.map(|ext| ext.trim_left_matches('.')).collect())
        .unwrap_or_else(|| vec!["js", "map", "jsbundle", "bundle"]);
    let ignores = matches
        .values_of("ignore")
        .map(|ignores| ignores.map(|i| format!("!{}", i)).collect::<Vec<_>>());
    let ignore_file = matches.value_of("ignore_file");
    let dist = matches.value_of("dist");

    let mut processor = SourceMapProcessor::new();

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

        let mut builder = WalkBuilder::new(path);
        builder.git_exclude(false).git_ignore(false).ignore(false);

        if check_ignore {
            let mut types_builder = TypesBuilder::new();
            for ext in &extensions {
                types_builder.add(ext, &format!("*.{}", ext))?;
            }
            builder.types(types_builder.select("all").build()?);

            if let Some(ignore_file) = ignore_file {
                // This could yield an optional partial error
                // We ignore this error to match behavior of git
                builder.add_ignore(ignore_file);
            }

            if let Some(ref ignores) = ignores {
                let mut override_builder = OverrideBuilder::new(path);
                for ignore in ignores {
                    override_builder.add(&ignore)?;
                }
                builder.overrides(override_builder.build()?);
            }
        }

        for result in builder.build() {
            let file = result?;
            if file.file_type().map_or(false, |t| t.is_dir()) {
                continue;
            }

            info!(
                "found: {} ({} bytes)",
                file.path().display(),
                file.metadata().unwrap().len()
            );
            let local_path = file.path().strip_prefix(&base_path).unwrap();
            let url = format!("{}/{}{}", url_prefix, path_as_url(local_path), url_suffix);
            processor.add(&url, file.path())?;
        }
    }

    if matches.is_present("rewrite") {
        let mut prefixes: Vec<&str> = match matches.values_of("strip_prefix") {
            Some(paths) => paths.collect(),
            None => vec![],
        };
        if matches.is_present("strip_common_prefix") {
            prefixes.push("~");
        }
        processor.rewrite(&prefixes)?;
    }

    if !matches.is_present("no_sourcemap_reference") {
        processor.add_sourcemap_references()?;
    }

    if matches.is_present("validate") {
        processor.validate_all()?;
    }

    let org = ctx.get_org()?;

    // make sure the release exists
    let release = ctx.api.new_release(
        &org,
        &NewRelease {
            version: version.into(),
            projects: ctx.get_projects(matches)?,
            ..Default::default()
        },
    )?;

    let project = ctx.get_project_default().ok();
    processor.upload(
        &ctx.api,
        org,
        project.as_ref().map(|x| x.as_str()),
        &release.version,
        dist,
    )?;

    Ok(())
}

fn execute_files<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
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
    ctx: &ReleaseContext,
    matches: &ArgMatches<'a>,
    version: &str,
) -> Result<(), Error> {
    let mut deploy = Deploy {
        env: matches.value_of("env").unwrap().to_string(),
        name: matches.value_of("name").map(|x| x.to_string()),
        url: matches.value_of("url").map(|x| x.to_string()),
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
    let mut name = deploy.name.as_ref().map(|x| x.as_str()).unwrap_or("");
    if name == "" {
        name = "unnamed";
    }

    println!("Created new deploy {} for '{}'", name, deploy.env);

    Ok(())
}

fn execute_deploys_list<'a>(
    ctx: &ReleaseContext,
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
        let mut name = deploy.name.as_ref().map(|x| x.as_str()).unwrap_or("");
        if name == "" {
            name = "unnamed";
        }
        table.add_row().add(deploy.env).add(name).add(HumanDuration(
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

fn execute_deploys<'a>(ctx: &ReleaseContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let release = matches.value_of("version").unwrap();
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_deploys_new(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_deploys_list(ctx, sub_matches, release);
    }
    unreachable!();
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    // this one does not need a context or org
    if let Some(_sub_matches) = matches.subcommand_matches("propose-version") {
        return execute_propose_version();
    }

    let config = Config::get_current();
    let ctx = ReleaseContext {
        api: Api::get_current(),
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
