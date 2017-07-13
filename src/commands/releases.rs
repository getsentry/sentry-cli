//! Implements a command for managing releases.
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::collections::HashSet;

use clap::{App, AppSettings, Arg, ArgMatches};
use walkdir::WalkDir;
use chrono::{DateTime, Duration, UTC};
use regex::Regex;

use prelude::*;
use api::{Api, NewRelease, UpdatedRelease, FileContents, Deploy};
use config::Config;
use indicatif::HumanBytes;
use utils::{ArgExt, Table, HumanDuration, validate_timestamp,
            validate_seconds, get_timestamp, validate_project,
            SourceMapProcessor, vcs, detect_release_name};


struct ReleaseContext<'a> {
    pub api: Api<'a>,
    pub config: &'a Config,
    pub org: String,
    pub project_default: Option<&'a str>,
}

impl<'a> ReleaseContext<'a> {
    pub fn get_org(&'a self) -> Result<&str> {
        Ok(&self.org)
    }

    pub fn get_project_default(&'a self) -> Result<String> {
        if let Some(ref proj) = self.project_default {
            Ok(proj.to_string())
        } else {
            Ok(self.config.get_project_default()?)
        }
    }

    pub fn get_projects(&'a self, matches: &ArgMatches<'a>) -> Result<Vec<String>> {
        if let Some(projects) = matches.values_of("projects") {
             Ok(projects.map(|x| x.to_string()).collect())       
        } else if let Some(project) = self.project_default {
            Ok(vec![project.to_string()])
        } else {
            Ok(vec![self.config.get_project_default()?])
        }
    }
}


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("manage releases on Sentry")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_arg()
        .arg(Arg::with_name("project")
            .hidden(true)
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .validator(validate_project))
        .subcommand(App::new("new")
            .about("Create a new release")
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
                .help("Optional URL to the release for information purposes"))
            .arg(Arg::with_name("finalize")
                 .long("finalize")
                 .help("Immediately finalize the release (sets it to released)")))
        .subcommand(App::new("propose-version")
            .about("Proposes a version name for a new release"))
        .subcommand(App::new("set-commits")
            .about("Sets commits to a release")
            .version_arg(1)
            .arg(Arg::with_name("clear")
                 .long("clear")
                 .help("If this is passed the commits will be cleared from the release."))
            .arg(Arg::with_name("auto")
                 .long("auto")
                 .help("This parameter enables completely automated commit management. \
                        It requires that the command is run from within a git repository. \
                        sentry-cli will then automatically find remotely configured \
                        repositories and discover commits."))
            .arg(Arg::with_name("commits")
                 .long("commit")
                 .short("c")
                 .value_name("SPEC")
                 .multiple(true)
                 .number_of_values(1)
                 .help("This parameter defines a single commit for a repo as \
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
            .about("Delete a release")
            .version_arg(1))
        .subcommand(App::new("finalize")
            .about("Marks a release as finalized and released.")
            .version_arg(1)
            .arg(Arg::with_name("started")
                 .long("started")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("If set the release start date is set to this value."))
            .arg(Arg::with_name("released")
                 .long("released")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("The releaes time (if not provided the current time is used).")))
        .subcommand(App::new("list")
            .about("list the most recent releases")
            .arg(Arg::with_name("no_abbrev")
                .long("no-abbrev")
                .help("Do not abbreviate the release version")))
        .subcommand(App::new("info")
            .about("Return information about a release")
            .version_arg(1)
            .arg(Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Do not print any output.  If this is passed the command can be \
                       used to determine if a release already exists.  The exit status \
                       will be 0 if the release exists or 1 otherwise.")))
        .subcommand(App::new("files")
            .about("manage release artifact files")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .version_arg(1)
            .subcommand(App::new("list").about("List all release files"))
            .subcommand(App::new("delete")
                .about("Delete a release file")
                .arg(Arg::with_name("all")
                    .short("A")
                    .long("all")
                    .help("deletes all files"))
                .arg(Arg::with_name("names")
                    .value_name("NAMES")
                    .index(1)
                    .multiple(true)
                    .help("a list of filenames to delete.")))
            .subcommand(App::new("upload")
                .about("Uploads a file for a given release")
                .arg(Arg::with_name("dist")
                    .long("dist")
                    .short("d")
                    .value_name("DISTRIBUTION")
                    .help("Optional distribution identifier for this file"))
                .arg(Arg::with_name("headers")
                    .long("header")
                    .short("H")
                    .value_name("KEY VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Stores a header with this file"))
                .arg(Arg::with_name("path")
                    .value_name("PATH")
                    .index(1)
                    .required(true)
                    .help("The file to upload"))
                .arg(Arg::with_name("name")
                    .index(2)
                    .value_name("NAME")
                    .help("The name of the file on the server.")))
            .subcommand(App::new("upload-sourcemaps")
                .about("Uploads sourcemap information for a given release")
                .arg(Arg::with_name("paths")
                    .value_name("PATHS")
                    .index(1)
                    .required(true)
                    .multiple(true)
                    .help("The files to upload"))
                .arg(Arg::with_name("url_prefix")
                    .short("u")
                    .long("url-prefix")
                    .value_name("PREFIX")
                    .help("The URL prefix to prepend to all filenames"))
                .arg(Arg::with_name("dist")
                    .long("dist")
                    .short("d")
                    .value_name("DISTRIBUTION")
                    .help("Optional distribution identifier for the sourcemaps"))
                .arg(Arg::with_name("validate")
                    .long("validate")
                    .help("Enable basic sourcemap validation"))
                .arg(Arg::with_name("no_sourcemap_reference")
                    .long("no-sourcemap-reference")
                    .help("Disables the emitting of automatic sourcemap references. \
                            By default the tool will store a 'Sourcemap' header with \
                            minified files so that sourcemaps are located automatically \
                            if the tool can detect a link. If this causes issues it can \
                            be disabled."))
                .arg(Arg::with_name("rewrite")
                    .long("rewrite")
                    .help("Enables rewriting of matching sourcemaps \
                            so that indexed maps are flattened and missing \
                            sources are inlined if possible.  This fundamentally \
                            changes the upload process to be based on sourcemaps \
                            and minified files exclusively and comes in handy for \
                            setups like react-native that generate sourcemaps that \
                            would otherwise not work for sentry."))
                .arg(Arg::with_name("strip_prefix")
                    .long("strip-prefix")
                    .value_name("PREFIX")
                    .multiple(true)
                    .number_of_values(1)
                    .help("When passed all sources that start with the given prefix \
                            will have that prefix stripped from the filename.  This \
                            requires --rewrite to be enabled."))
                .arg(Arg::with_name("strip_common_prefix")
                    .long("strip-common-prefix")
                    .help("Similar to --strip-prefix but strips the most common \
                            prefix on all sources."))
                // legacy parameter
                .arg(Arg::with_name("verbose")
                    .long("verbose")
                    .short("verbose")
                    .hidden(true))
                .arg(Arg::with_name("extensions")
                    .long("ext")
                    .short("x")
                    .value_name("EXT")
                    .multiple(true)
                    .number_of_values(1)
                    .help("Add a file extension to the list of files to upload."))))
        .subcommand(App::new("deploys")
            .about("Manages deploys for a release")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .version_arg(1)
            .subcommand(App::new("new")
                .about("Creates a deploy for a release")
                .arg(Arg::with_name("env")
                     .long("env")
                     .short("e")
                     .value_name("ENV")
                     .required(true)
                     .help("This sets the environment for this release.  This needs to be \
                            provided.  Values that make sense here would be 'production' or \
                            'staging'."))
                .arg(Arg::with_name("name")
                     .long("name")
                     .short("n")
                     .value_name("NAME")
                     .help("An optional human visible name for this deploy."))
                .arg(Arg::with_name("url")
                     .long("url")
                     .short("u")
                     .value_name("URL")
                     .help("An optional optional URL that points to the deployment."))
                .arg(Arg::with_name("started")
                     .long("started")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deploy was started."))
                .arg(Arg::with_name("finished")
                     .long("finished")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deploy was finished."))
                .arg(Arg::with_name("time")
                     .long("time")
                     .short("t")
                     .value_name("SECONDS")
                     .validator(validate_seconds)
                     .help("Alternatively to `--started` and `--finished` an optional \
                            time in seconds that indicates how long it took for the \
                            deploy to finish.")))
            .subcommand(App::new("list")
                .about("List all deploys of a release")))
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
        static ref DOTTED_PATH_PREFIX_RE: Regex = Regex::new(
            r"^([a-z][a-z0-9-]+)(\.[a-z][a-z0-9-]+)+-").unwrap();
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

fn execute_new<'a>(ctx: &ReleaseContext,
                   matches: &ArgMatches<'a>) -> Result<()> {
    let info_rv = ctx.api.new_release(ctx.get_org()?,
        &NewRelease {
            version: matches.value_of("version").unwrap().to_owned(),
            projects: ctx.get_projects(matches)?,
            url: matches.value_of("url").map(|x| x.to_owned()),
            date_started: Some(UTC::now()),
            date_released: if matches.is_present("finalize") {
                Some(UTC::now())
            } else {
                None
            },
            ..Default::default()
        })?;
    println!("Created release {}.", info_rv.version);
    Ok(())
}

fn execute_finalize<'a>(ctx: &ReleaseContext,
                        matches: &ArgMatches<'a>) -> Result<()> {
    fn get_date(value: Option<&str>, now_default: bool) -> Result<Option<DateTime<UTC>>> {
        match value {
            None => Ok(if now_default { Some(UTC::now()) } else { None }),
            Some(value) => Ok(Some(get_timestamp(value)?))
        }
    }

    let info_rv = ctx.api.update_release(ctx.get_org()?,
        matches.value_of("version").unwrap(),
        &UpdatedRelease {
            projects: Some(ctx.get_projects(matches)?),
            url: matches.value_of("url").map(|x| x.to_owned()),
            date_started: get_date(matches.value_of("started"), false)?,
            date_released: get_date(matches.value_of("released"), true)?,
            ..Default::default()
        })?;
    println!("Finalized release {}.", info_rv.version);
    Ok(())
}

fn execute_propose_version() -> Result<()>
{
    println!("{}", detect_release_name()?);
    Ok(())
}

fn execute_set_commits<'a>(ctx: &ReleaseContext,
                           matches: &ArgMatches<'a>) -> Result<()>
{
    let version = matches.value_of("version").unwrap();
    let org = ctx.get_org()?;
    let repos = ctx.api.list_organization_repos(org)?;
    let mut commit_specs = vec![];

    if repos.is_empty() {
        return Err(Error::from("No repositories are configured in Sentry for \
                                your organization."));
    }

    let heads = if matches.is_present("auto") {
        let commits = vcs::find_heads(None, repos)?;
        if commits.is_empty() {
            return Err(Error::from("Could not determine any commits to be associated \
                                    automatically. You will have to explicitly provide \
                                    commits on the command line."));
        }
        Some(commits)
    } else if matches.is_present("clear") {
        Some(vec![])
    } else {
        if let Some(commits) = matches.values_of("commits") {
            for spec in commits {
                let commit_spec = vcs::CommitSpec::parse(spec)?;
                if (&repos).iter().filter(|r| r.name == commit_spec.repo).next().is_some() {
                    commit_specs.push(commit_spec);
                } else {
                    return Err(Error::from(format!("Unknown repo '{}'", commit_spec.repo)));
                }
            }
        }
        let commits = vcs::find_heads(Some(commit_specs), repos)?;
        if commits.is_empty() {
            None
        } else {
            Some(commits)
        }
    };

    // make sure the release exists if projects are given
    if let Ok(projects) = ctx.get_projects(matches) {
        ctx.api.new_release(&org, &NewRelease {
            version: version.into(),
            projects: projects,
            ..Default::default()
        })?;
    }

    if let Some(heads) = heads {
        if heads.is_empty() {
            println!("Clearing commits for release.");
        } else {
            let mut table = Table::new();
            table.title_row()
                .add("Repository")
                .add("Revision");
            for commit in &heads {
                let mut row = table.add_row();
                row.add(&commit.repo);
                if let Some(ref prev_rev) = commit.prev_rev {
                    row.add(format!("{} -> {}", strip_sha(prev_rev), strip_sha(&commit.rev)));
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

fn execute_delete<'a>(ctx: &ReleaseContext,
                      matches: &ArgMatches<'a>) -> Result<()> {
    let version = matches.value_of("version").unwrap();
    let project = ctx.get_project_default().ok();
    if ctx.api.delete_release(ctx.get_org()?, project.as_ref().map(|x| x.as_str()), version)? {
        println!("Deleted release {}!", version);
    } else {
        println!("Did nothing. Release with this version ({}) does not exist.",
                 version);
    }
    Ok(())
}

fn execute_list<'a>(ctx: &ReleaseContext,
                    matches: &ArgMatches<'a>) -> Result<()> {
    let project = ctx.get_project_default().ok();
    let releases = ctx.api.list_releases(ctx.get_org()?, project.as_ref().map(|x| x.as_str()))?;
    let abbrev = !matches.is_present("no_abbrev");
    let mut table = Table::new();
    table.title_row()
        .add("Released")
        .add("Version")
        .add("New Events")
        .add("Last Event");
    for release_info in releases {
        let mut row = table.add_row();
        if let Some(date) = release_info.date_released {
            row.add(format!("{} ago", HumanDuration(UTC::now().signed_duration_since(date))));
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
            row.add(format!("{} ago", HumanDuration(UTC::now().signed_duration_since(date))));
        } else {
            row.add("-");
        }
    }
    table.print();
    Ok(())
}

fn execute_info<'a>(ctx: &ReleaseContext,
                    matches: &ArgMatches<'a>) -> Result<()> {
    let version = matches.value_of("version").unwrap();
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    let release = ctx.api.get_release(org, project.as_ref().map(|x| x.as_str()),
                                      &version)?;

    // quiet mode just exists
    if matches.is_present("quiet") {
        if release.is_none() {
            return Err(ErrorKind::QuietExit(1).into());
        }
        return Ok(());
    }

    if let Some(release) = release {
        let short_version = strip_version(&release.version);
        let mut tbl = Table::new();
        tbl.add_row()
            .add("Version")
            .add(short_version);
        if short_version != &release.version {
            tbl.add_row()
                .add("Full version")
                .add(&release.version);
        }
        tbl.add_row()
            .add("Date created")
            .add(&release.date_created);
        if let Some(last_event) = release.last_event {
            tbl.add_row()
                .add("Last event")
                .add(last_event);
        }
        tbl.print();
    } else {
        println!("No such release");
        return Err(ErrorKind::QuietExit(1).into());
    }
    Ok(())
}

fn execute_files_list<'a>(ctx: &ReleaseContext,
                          _matches: &ArgMatches<'a>,
                          release: &str)
                          -> Result<()> {
    let mut table = Table::new();
    table.title_row()
        .add("Name")
        .add("Distribution")
        .add("Source Map")
        .add("Size");

    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    for artifact in ctx.api.list_release_files(
            org, project.as_ref().map(|x| x.as_str()), release)? {
        let mut row = table.add_row();
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

fn execute_files_delete<'a>(ctx: &ReleaseContext,
                            matches: &ArgMatches<'a>,
                            release: &str)
                            -> Result<()> {
    let files: HashSet<String> = match matches.values_of("names") {
        Some(paths) => paths.map(|x| x.into()).collect(),
        None => HashSet::new(),
    };
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    for file in ctx.api.list_release_files(org, project.as_ref().map(|x| x.as_str()), release)? {
        if !(matches.is_present("all") || files.contains(&file.name)) {
            continue;
        }
        if ctx.api.delete_release_file(org, project.as_ref().map(|x| x.as_str()), release, &file.id)? {
            println!("D {}", file.name);
        }
    }
    Ok(())
}

fn execute_files_upload<'a>(ctx: &ReleaseContext,
                            matches: &ArgMatches<'a>,
                            version: &str)
                            -> Result<()> {
    let path = Path::new(matches.value_of("path").unwrap());
    let name = match matches.value_of("name") {
        Some(name) => name,
        None => {
            Path::new(path).file_name()
                .and_then(|x| x.to_str())
                .ok_or("No filename provided.")?
        }
    };
    let dist = matches.value_of("dist");
    let mut headers = vec![];
    if let Some(header_list) = matches.values_of("header") {
        for header in header_list {
            if !header.contains(':') {
                fail!("Invalid header. Needs to be in key:value format");
            }
            let mut iter = header.splitn(2, ':');
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    };
    let org = ctx.get_org()?;
    let project = ctx.get_project_default().ok();
    if let Some(artifact) = ctx.api.upload_release_file(org,
            project.as_ref().map(|x| x.as_str()), &version,
            FileContents::FromPath(&path), &name, dist, Some(&headers[..]))? {
        println!("A {}  ({} bytes)", artifact.sha1, artifact.size);
    } else {
        fail!("File already present!");
    }
    Ok(())
}

fn execute_files_upload_sourcemaps<'a>(ctx: &ReleaseContext,
                                       matches: &ArgMatches<'a>,
                                       version: &str)
                                       -> Result<()> {
    let url_prefix = matches.value_of("url_prefix").unwrap_or("~").trim_right_matches("/");
    let paths = matches.values_of("paths").unwrap();
    let extensions = match matches.values_of("extensions") {
        Some(matches) => matches.map(|ext| OsStr::new(ext.trim_left_matches("."))).collect(),
        None => {
            vec![OsStr::new("js"), OsStr::new("map"), OsStr::new("jsbundle"), OsStr::new("bundle")]
        }
    };
    let dist = matches.value_of("dist");

    let mut processor = SourceMapProcessor::new();

    for path in paths {
        // if we start walking over something that is an actual file then
        // the directory iterator yields that path and terminates.  We
        // handle that case here specifically to figure out what the path is
        // we should strip off.
        let walk_path = PathBuf::from(&path);
        let (base_path, skip_ext_test) = if walk_path.is_file() {
            (walk_path.parent().unwrap(), true)
        } else {
            (walk_path.as_path(), false)
        };

        for dent in WalkDir::new(&walk_path) {
            let dent = dent?;
            if !skip_ext_test {
                let extension = dent.path().extension();
                if !extensions.iter().any(|ext| Some(*ext) == extension) {
                    continue;
                }
            }
            info!("found: {} ({} bytes)",
                  dent.path().display(),
                  dent.metadata().unwrap().len());
            let local_path = dent.path().strip_prefix(&base_path).unwrap();
            let url = format!("{}/{}", url_prefix, path_as_url(local_path));
            processor.add(&url, dent.path())?;
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
    let release = ctx.api.new_release(&org, &NewRelease {
        version: version.into(),
        projects: ctx.get_projects(matches)?,
        ..Default::default()
    })?;

    let project = ctx.get_project_default().ok();
    processor.upload(&ctx.api, org, project.as_ref().map(|x| x.as_str()),
                     &release.version, dist)?;

    Ok(())
}

fn execute_files<'a>(ctx: &ReleaseContext,
                     matches: &ArgMatches<'a>)
                     -> Result<()> {
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

fn execute_deploys_new<'a>(ctx: &ReleaseContext,
                           matches: &ArgMatches<'a>,
                           version: &str)
    -> Result<()>
{
    let mut deploy = Deploy {
        env: matches.value_of("env").unwrap().to_string(),
        name: matches.value_of("name").map(|x| x.to_string()),
        url: matches.value_of("url").map(|x| x.to_string()),
        ..Default::default()
    };

    if let Some(value) = matches.value_of("time") {
        let finished = UTC::now();
        deploy.finished = Some(finished);
        deploy.started = Some(finished - Duration::seconds(value.parse().unwrap()));
    } else {
        if let Some(finished_str) = matches.value_of("finished") {
            deploy.finished = Some(get_timestamp(finished_str)?);
        } else {
            deploy.finished = Some(UTC::now());
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

fn execute_deploys_list<'a>(ctx: &ReleaseContext,
                            _matches: &ArgMatches<'a>,
                            version: &str)
    -> Result<()>
{
    let mut table = Table::new();
    table.title_row()
        .add("Environment")
        .add("Name")
        .add("Finished");

    for deploy in ctx.api.list_deploys(ctx.get_org()?, version)? {
        let mut name = deploy.name.as_ref().map(|x| x.as_str()).unwrap_or("");
        if name == "" {
            name = "unnamed";
        }
        table.add_row()
            .add(deploy.env)
            .add(name)
            .add(HumanDuration(UTC::now().signed_duration_since(deploy.finished.unwrap())));
    }

    if table.is_empty() {
        println!("No deploys found");
    } else {
        table.print();
    }

    Ok(())
}

fn execute_deploys<'a>(ctx: &ReleaseContext,
                       matches: &ArgMatches<'a>) -> Result<()> {
    let release = matches.value_of("version").unwrap();
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_deploys_new(ctx, sub_matches, release);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_deploys_list(ctx, sub_matches, release);
    }
    unreachable!();
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let ctx = ReleaseContext {
        api: Api::new(config),
        config: config,
        org: config.get_org(matches)?,
        project_default: matches.value_of("project"),
    };
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_new(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("finalize") {
        return execute_finalize(&ctx, sub_matches);
    }
    if let Some(_sub_matches) = matches.subcommand_matches("propose-version") {
        return execute_propose_version();
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
