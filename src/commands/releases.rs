//! Implements a command for managing releases.
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::{bail, err_msg, Error};
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use indicatif::HumanBytes;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use regex::Regex;

use crate::api::{Api, Deploy, FileContents, NewRelease, ProgressBarMode, UpdatedRelease};
use crate::config::Config;
use crate::utils::args::{
    get_timestamp, validate_project, validate_seconds, validate_timestamp, ArgExt,
};
use crate::utils::formatting::{HumanDuration, Table};
use crate::utils::releases::detect_release_name;
use crate::utils::sourcemaps::{SourceMapProcessor, UploadContext};
use crate::utils::system::QuietExit;
use crate::utils::vcs::{find_heads, CommitSpec};

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
        if let Some(ref proj) = self.project_default {
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
                .hidden(true)
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
                .hidden(true)))
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
                .about("Upload a file for a release.")
                .arg(Arg::with_name("dist")
                    .long("dist")
                    .short("d")
                    .value_name("DISTRIBUTION")
                    .help("Optional distribution identifier for this file."))
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
                    .help("The path to the file to upload."))
                .arg(Arg::with_name("name")
                    .index(2)
                    .value_name("NAME")
                    .help("The name of the file on the server.")))
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
        .arg(
            Arg::with_name("wait")
                .long("wait")
                .help("Wait for the server to fully process uploaded files."),
        )
                .arg(Arg::with_name("no_sourcemap_reference")
                    .long("no-sourcemap-reference")
                    .help("Disable emitting of automatic sourcemap references.{n}\
                           By default the tool will store a 'Sourcemap' header with \
                           minified files so that sourcemaps are located automatically \
                           if the tool can detect a link. If this causes issues it can \
                           be disabled."))
                .arg(Arg::with_name("no_rewrite")
                    .long("no-rewrite")
                    .help("The opposite of --rewrite. By default sourcemaps are not rewritten.")
                    .conflicts_with("rewrite"))
                .arg(Arg::with_name("rewrite")
                    .long("rewrite")
                    .help("Enables rewriting of matching sourcemaps \
                           so that indexed maps are flattened and missing \
                           sources are inlined if possible.{n}This fundamentally \
                           changes the upload process to be based on sourcemaps \
                           and minified files exclusively and comes in handy for \
                           setups like react-native that generate sourcemaps that \
                           would otherwise not work for sentry.")
                    .conflicts_with("no_rewrite"))
                .arg(Arg::with_name("strip_prefix")
                    .long("strip-prefix")
                    .value_name("PREFIX")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Strip the given prefix from all filenames.{n}\
                           Only files that start with the given prefix will be stripped."))
                .arg(Arg::with_name("strip_common_prefix")
                    .long("strip-common-prefix")
                    .help("Similar to --strip-prefix but strips the most common \
                           prefix on all sources."))
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
                    .help("Add a file extension to the list of files to upload."))))
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
                     .validator(validate_seconds)
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

    if repos.is_empty() {
        bail!(
            "No repositories are configured in Sentry for \
             your organization."
        );
    }

    let heads = if matches.is_present("auto") {
        let commits = find_heads(None, &repos)?;
        if commits.is_empty() {
            let config = Config::current();

            bail!(
                "Could not determine any commits to be associated automatically.\n\
                 Please provide commits explicitly using --commit | -c.\n\
                 \n\
                 HINT: Did you add the repo to your organization?\n\
                 Configure it at {}/settings/{}/repos/",
                config.get_base_url()?,
                org
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

fn execute_delete<'a>(ctx: &ReleaseContext<'_>, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let version = matches.value_of("version").unwrap();
    let project = ctx.get_project_default().ok();
    if ctx.api.delete_release(
        ctx.get_org()?,
        project.as_ref().map(String::as_ref),
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

fn execute_list<'a>(ctx: &ReleaseContext<'_>, _matches: &ArgMatches<'a>) -> Result<(), Error> {
    let project = ctx.get_project_default().ok();
    let releases = ctx
        .api
        .list_releases(ctx.get_org()?, project.as_ref().map(String::as_ref))?;
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
        row.add(&release_info.version);
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
    let release = ctx
        .api
        .get_release(org, project.as_ref().map(String::as_ref), &version)?;

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
    for artifact in
        ctx.api
            .list_release_files(org, project.as_ref().map(String::as_ref), release)?
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
        .list_release_files(org, project.as_ref().map(String::as_ref), release)?
    {
        if !(matches.is_present("all") || files.contains(&file.name)) {
            continue;
        }
        if ctx.api.delete_release_file(
            org,
            project.as_ref().map(String::as_ref),
            release,
            &file.id,
        )? {
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
    let path = Path::new(matches.value_of("path").unwrap());
    let name = match matches.value_of("name") {
        Some(name) => name,
        None => Path::new(path)
            .file_name()
            .and_then(OsStr::to_str)
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
        project.as_ref().map(String::as_ref),
        &version,
        &FileContents::FromPath(&path),
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

fn get_url_prefix_from_args<'a, 'b>(matches: &'b ArgMatches<'a>) -> &'b str {
    matches
        .value_of("url_prefix")
        .unwrap_or("~")
        .trim_end_matches('/')
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

    processor.add(&bundle_url, &bundle_path)?;
    processor.add(&sourcemap_url, &sourcemap_path)?;

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
    let extensions = matches
        .values_of("extensions")
        .map(|extensions| extensions.map(|ext| ext.trim_start_matches('.')).collect())
        .unwrap_or_else(|| vec!["js", "map", "jsbundle", "bundle"]);
    let ignores = matches
        .values_of("ignore")
        .map(|ignores| ignores.map(|i| format!("!{}", i)).collect::<Vec<_>>());
    let ignore_file = matches.value_of("ignore_file");

    let url_prefix = get_url_prefix_from_args(matches);
    let url_suffix = get_url_suffix_from_args(matches);

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

    // We want to change the default from --no-rewrite to --rewrite, but we need to transition
    // users to explicitly pass the option first such that we don't break their setup when we do.
    if !matches.is_present("no_rewrite") && !matches.is_present("rewrite") {
        warn!(
            "The default --no-rewrite will disappear. Please specify --rewrite or \
             --no-rewrite explicitly during sourcemap upload."
        )
    }

    if matches.is_present("rewrite") {
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
        project: project.as_ref().map(String::as_str),
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
    let mut name = deploy.name.as_ref().map(String::as_ref).unwrap_or("");
    if name == "" {
        name = "unnamed";
    }

    println!("Created new deploy {} for '{}'", name, deploy.env);

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
        let mut name = deploy.name.as_ref().map(String::as_ref).unwrap_or("");
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

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
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
