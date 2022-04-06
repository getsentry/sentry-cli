use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use clap::{Arg, ArgMatches, Command};
use lazy_static::lazy_static;
use regex::Regex;

use crate::api::{
    Api, Deploy, NewRelease, NoneReleaseInfo, OptionalReleaseInfo, ReleaseStatus, UpdatedRelease,
};
use crate::config::Config;
use crate::utils::args::{get_timestamp, validate_int, validate_timestamp, ArgExt};
use crate::utils::formatting::{HumanDuration, Table};
use crate::utils::logging::is_quiet_mode;
use crate::utils::releases::detect_release_name;
use crate::utils::system::QuietExit;
use crate::utils::vcs::{
    find_heads, generate_patch_set, get_commits_from_git, get_repo_from_remote, CommitSpec,
};

pub fn make_command(command: Command) -> Command {
    command.about("Manage releases on Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .project_arg(true)
        // Backwards compatibility with `releases files <VERSION>` commands.
        .subcommand(crate::commands::files::make_command(Command::new("files")).version_arg().hide(true))
        .subcommand(Command::new("new")
            .about("Create a new release.")
            .version_arg()
            .arg(Arg::new("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."))
            .arg(Arg::new("finalize")
                 .long("finalize")
                 .help("Immediately finalize the release. (sets it to released)")))
        .subcommand(Command::new("propose-version")
            .about("Propose a version name for a new release."))
        .subcommand(Command::new("set-commits")
            .about("Set commits of a release.")
            .version_arg()
            .arg(Arg::new("clear")
                 .long("clear")
                 .help("Clear all current commits from the release."))
            .arg(Arg::new("auto")
                .long("auto")
                .help("Enable completely automated commit management.{n}\
                        This requires that the command is run from within a git repository.  \
                        sentry-cli will then automatically find remotely configured \
                        repositories and discover commits."))
            .arg(Arg::new("ignore-missing")
                .long("ignore-missing")
                .help("When the flag is set and the previous release commit was not found in the repository, \
                        will create a release with the default commits count (or the one specified with `--initial-depth`) \
                        instead of failing the command."))
            .arg(Arg::new("local")
                .conflicts_with_all(&["auto", "clear", "commits", ])
                .long("local")
                .help("Set commits of a release from local git.{n}\
                        This requires that the command is run from within a git repository.  \
                        sentry-cli will then automatically find remotely configured \
                        repositories and discover commits."))
            .arg(Arg::new("initial-depth")
                .conflicts_with("auto")
                .long("initial-depth")
                .value_name("INITIAL DEPTH")
                .validator(validate_int)
                .help("Set the number of commits of the initial release. The default is 20."))
            .arg(Arg::new("commits")
                 .long("commit")
                 .short('c')
                 .value_name("SPEC")
                 .multiple_occurrences(true)

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
        .subcommand(Command::new("delete")
            .about("Delete a release.")
            .version_arg())
        .subcommand(Command::new("archive")
            .about("Archive a release.")
            .version_arg())
        .subcommand(Command::new("restore")
            .about("Restore a release.")
            .version_arg())
        .subcommand(Command::new("finalize")
            .about("Mark a release as finalized and released.")
            .version_arg()
            .arg(Arg::new("url")
                .long("url")
                .value_name("URL")
                .help("Optional URL to the release for information purposes."))
            .arg(Arg::new("started")
                 .long("started")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("Set the release start date."))
            .arg(Arg::new("released")
                 .long("released")
                 .validator(validate_timestamp)
                 .value_name("TIMESTAMP")
                 .help("Set the release time. [defaults to the current time]")))
        .subcommand(Command::new("list")
            .about("List the most recent releases.")
            .arg(Arg::new("show_projects")
                .short('P')
                .long("show-projects")
                .help("Display the Projects column"))
            .arg(Arg::new("raw")
                .short('R')
                .long("raw")
                .help("Print raw, delimiter separated list of releases. [defaults to new line]"))
            .arg(Arg::new("delimiter")
                .short('D')
                .long("delimiter")
                .takes_value(true)
                .requires("raw")
                .help("Delimiter for the --raw flag")))
        .subcommand(Command::new("info")
            .about("Print information about a release.")
            .version_arg()
            .arg(Arg::new("show_projects")
                .short('P')
                .long("show-projects")
                .help("Display the Projects column"))
            .arg(Arg::new("show_commits")
                .short('C')
                .long("show-commits")
                .help("Display the Commits column")))
        .subcommand(Command::new("deploys")
            .about("Manage release deployments.")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .version_arg()
            .subcommand(Command::new("new")
                .about("Creates a new release deployment.")
                .arg(Arg::new("env")
                     .long("env")
                     .short('e')
                     .value_name("ENV")
                     .required(true)
                     .help("Set the environment for this release.{n}This argument is required.  \
                            Values that make sense here would be 'production' or 'staging'."))
                .arg(Arg::new("name")
                     .long("name")
                     .short('n')
                     .value_name("NAME")
                     .help("Optional human readable name for this deployment."))
                .arg(Arg::new("url")
                     .long("url")
                     .short('u')
                     .value_name("URL")
                     .help("Optional URL that points to the deployment."))
                .arg(Arg::new("started")
                     .long("started")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deployment started."))
                .arg(Arg::new("finished")
                     .long("finished")
                     .value_name("TIMESTAMP")
                     .validator(validate_timestamp)
                     .help("Optional unix timestamp when the deployment finished."))
                .arg(Arg::new("time")
                     .long("time")
                     .short('t')
                     .value_name("SECONDS")
                     .validator(validate_int)
                     .help("Optional deployment duration in seconds.{n}\
                            This can be specified alternatively to `--started` and `--finished`.")))
            .subcommand(Command::new("list")
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

fn execute_new(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();

    api.new_release(
        &config.get_org(matches)?,
        &NewRelease {
            version: version.to_owned(),
            projects: config.get_projects(matches)?,
            url: matches.value_of("url").map(str::to_owned),
            date_started: Some(Utc::now()),
            date_released: if matches.is_present("finalize") {
                Some(Utc::now())
            } else {
                None
            },
        },
    )?;

    println!("Created release {}.", version);
    Ok(())
}

fn execute_finalize(matches: &ArgMatches) -> Result<()> {
    fn get_date(value: Option<&str>, now_default: bool) -> Result<Option<DateTime<Utc>>> {
        match value {
            None => Ok(if now_default { Some(Utc::now()) } else { None }),
            Some(value) => Ok(Some(get_timestamp(value)?)),
        }
    }

    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();

    api.update_release(
        &config.get_org(matches)?,
        version,
        &UpdatedRelease {
            projects: config.get_projects(matches).ok(),
            url: matches.value_of("url").map(str::to_owned),
            date_started: get_date(matches.value_of("started"), false)?,
            date_released: get_date(matches.value_of("released"), true)?,
            ..Default::default()
        },
    )?;

    println!("Finalized release {}.", version);
    Ok(())
}

fn execute_propose_version(_matches: &ArgMatches) -> Result<()> {
    println!("{}", detect_release_name()?);
    Ok(())
}

fn execute_set_commits(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();
    let org = config.get_org(matches)?;
    let repos = api.list_organization_repos(&org)?;
    let mut commit_specs = vec![];

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
    if let Ok(projects) = config.get_projects(matches) {
        api.new_release(
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
        api.set_release_refs(&org, version, heads)?;
    } else {
        let default_count = matches
            .value_of("initial-depth")
            .unwrap_or("20")
            .parse::<usize>()?;

        if matches.is_present("auto") {
            println!("Could not determine any commits to be associated with a repo-based integration. Proceeding to find commits from local git tree.");
        }
        // Get the commit of the most recent release.
        let prev_commit = match api.get_previous_release_with_commits(&org, version)? {
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
            println!("No commits found. Leaving release alone. If you believe there should be some, change commits range or initial depth and try again.");
            return Ok(());
        }

        api.update_release(
            &config.get_org(matches)?,
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

fn execute_delete(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();
    let project = config.get_project(matches).ok();

    if api.delete_release(&config.get_org(matches)?, project.as_deref(), version)? {
        println!("Deleted release {}!", version);
    } else {
        println!(
            "Did nothing. Release with this version ({}) does not exist.",
            version
        );
    }

    Ok(())
}

fn execute_archive(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();

    let info_rv = api.update_release(
        &config.get_org(matches)?,
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

fn execute_restore(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();

    let info_rv = api.update_release(
        &config.get_org(matches)?,
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

fn execute_list(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let project = config.get_project(matches).ok();
    let releases = api.list_releases(&config.get_org(matches)?, project.as_deref())?;

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

fn execute_info(matches: &ArgMatches) -> Result<()> {
    let api = Api::current();
    let version = matches.value_of("version").unwrap();
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches).ok();
    let release = api.get_release(&org, project.as_deref(), version)?;

    if is_quiet_mode() {
        if release.is_none() {
            return Err(QuietExit(1).into());
        }
        return Ok(());
    }

    if let Some(release) = release {
        let mut tbl = Table::new();
        let title_row = tbl.title_row().add("Version").add("Date created");

        if release.last_event.is_some() {
            title_row.add("Last event");
        }

        if matches.is_present("show_projects") {
            title_row.add("Projects");
        }

        if matches.is_present("show_commits") {
            title_row.add("Commits");
        }

        let data_row = tbl
            .add_row()
            .add(&release.version)
            .add(&release.date_created);

        if let Some(last_event) = release.last_event {
            data_row.add(last_event);
        }

        if matches.is_present("show_projects") {
            let project_slugs = release
                .projects
                .into_iter()
                .map(|p| p.slug)
                .collect::<Vec<_>>();
            if !project_slugs.is_empty() {
                data_row.add(project_slugs.join("\n"));
            } else {
                data_row.add("-");
            }
        }

        if matches.is_present("show_commits") {
            if let Ok(Some(commits)) = api.get_release_commits(&org, project.as_deref(), version) {
                if !commits.is_empty() {
                    data_row.add(
                        commits
                            .into_iter()
                            .map(|c| c.id)
                            .collect::<Vec<String>>()
                            .join("\n"),
                    );
                } else {
                    data_row.add("-");
                }
            } else {
                data_row.add("-");
            }
        }

        tbl.print();
    } else {
        return Err(QuietExit(1).into());
    }
    Ok(())
}

fn execute_deploys_new(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();
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

    let org = config.get_org(matches)?;
    let deploy = api.create_deploy(&org, version, &deploy)?;

    println!("Created new deploy {} for '{}'", deploy.name(), deploy.env);

    Ok(())
}

fn execute_deploys_list(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let api = Api::current();
    let version = matches.value_of("version").unwrap();
    let mut table = Table::new();
    table
        .title_row()
        .add("Environment")
        .add("Name")
        .add("Finished");

    for deploy in api.list_deploys(&config.get_org(matches)?, version)? {
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

fn execute_deploys(matches: &ArgMatches) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_deploys_new(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_deploys_list(sub_matches);
    }
    unreachable!();
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("propose-version") {
        return execute_propose_version(sub_matches);
    }

    if let Some(sub_matches) = matches.subcommand_matches("new") {
        return execute_new(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("finalize") {
        return execute_finalize(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("set-commits") {
        return execute_set_commits(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("delete") {
        return execute_delete(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("archive") {
        return execute_archive(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("restore") {
        return execute_restore(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_list(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("info") {
        return execute_info(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("files") {
        return crate::commands::files::execute(sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("deploys") {
        return execute_deploys(sub_matches);
    }
    unreachable!();
}
