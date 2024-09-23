use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use anyhow::{bail, Result};
use chrono::Duration;
use clap::{Arg, ArgAction, ArgMatches, Command};
use if_chain::if_chain;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::Api;
use crate::config::Config;
use crate::constants::DEFAULT_MAX_WAIT;
use crate::utils::args::{validate_distribution, ArgExt};
use crate::utils::file_search::ReleaseFileSearch;
use crate::utils::file_upload::UploadContext;
use crate::utils::fs::TempFile;
use crate::utils::sourcemaps::SourceMapProcessor;
use crate::utils::system::propagate_exit_status;
use crate::utils::xcode::{InfoPlist, MayDetach};

#[derive(Serialize, Deserialize, Default, Debug)]
struct SourceMapReport {
    packager_bundle_path: Option<PathBuf>,
    packager_sourcemap_path: Option<PathBuf>,
    hermes_bundle_path: Option<PathBuf>,
    hermes_sourcemap_path: Option<PathBuf>,
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Upload react-native projects in a Xcode build step.")
        .org_arg()
        .project_arg(false)
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .action(ArgAction::SetTrue)
                .help(
                    "Force the script to run, even in debug configuration.{n}This rarely \
                    does what you want because the default build script does not actually \
                    produce any information that the sentry build tool could pick up on.",
                ),
        )
        .arg(
            Arg::new("allow_fetch")
                .long("allow-fetch")
                .action(ArgAction::SetTrue)
                .help(
                    "Enable sourcemap fetching from the packager.{n}If this is enabled \
                    the react native packager needs to run and sourcemaps are downloade \
                    from it if the simulator platform is detected.",
                ),
        )
        .arg(
            Arg::new("fetch_from")
                .long("fetch-from")
                .value_name("URL")
                .help(
                    "Set the URL to fetch sourcemaps from.{n}\
                     The default is http://127.0.0.1:8081/, where the react-native \
                     packager runs by default.",
                ),
        )
        .arg(
            Arg::new("force_foreground")
                .long("force-foreground")
                .action(ArgAction::SetTrue)
                .help(
                    "Wait for the process to finish.{n}\
                     By default part of the build process will when triggered from Xcode \
                     detach and continue in the background.  When an error happens, \
                     a dialog is shown.  If this parameter is passed, Xcode will wait \
                     for the process to finish before the build finishes and output \
                     will be shown in the Xcode build output.",
                ),
        )
        .arg(Arg::new("build_script").value_name("BUILD_SCRIPT").help(
            "Optional path to the build script.{n}\
                     This is the path to the `react-native-xcode.sh` script you want \
                     to use.  By default the bundled build script is used.",
        ))
        .arg(
            Arg::new("dist")
                .long("dist")
                .value_name("DISTRIBUTION")
                .action(ArgAction::Append)
                .value_parser(validate_distribution)
                .help("The names of the distributions to publish. Can be supplied multiple times."),
        )
        .arg(
            Arg::new("args")
                .value_name("ARGS")
                .num_args(1..)
                .last(true)
                .help("Optional arguments to pass to the build script."),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .conflicts_with("wait_for")
                .help("Wait for the server to fully process uploaded files."),
        )
        .arg(
            Arg::new("wait_for")
                .long("wait-for")
                .value_name("SECS")
                .value_parser(clap::value_parser!(u64))
                .conflicts_with("wait")
                .help(
                    "Wait for the server to fully process uploaded files, \
                     but at most for the given number of seconds.",
                ),
        )
        .arg(
            Arg::new("no_auto_release")
                .long("no-auto-release")
                .action(ArgAction::SetTrue)
                .help("Don't try to automatically read release from Xcode project files."),
        )
}

fn find_node() -> String {
    if let Ok(path) = env::var("NODE_BINARY") {
        if !path.is_empty() {
            return path;
        }
    }
    "node".into()
}

fn find_hermesc() -> String {
    if let Ok(path) = env::var("HERMES_CLI_PATH") {
        if !path.is_empty() {
            return path;
        }
    }

    let pods_root_path = env::var("PODS_ROOT").unwrap_or("".to_string());
    format!("{}/hermes-engine/destroot/bin/hermesc", pods_root_path)
}

/// Check if Hermes is enabled based its executable existence in the installed pods
/// The same as RN Tooling does it https://github.com/facebook/react-native/blob/435245978122d34a78014600562517c3bf96f92e/scripts/react-native-xcode.sh#L98C11-L98C11
/// We ignore `USE_HERMES` as its behavior is not consistent between 0.65 - 0.72 and it the later versions it was removed as user override.
fn is_hermes_enabled(hermesc: &String) -> bool {
    return Path::new(hermesc).exists();
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let should_wrap = matches.get_flag("force")
        || match env::var("CONFIGURATION") {
            Ok(config) => !&config.contains("Debug"),
            Err(_) => bail!("Need to run this from Xcode"),
        };
    let base = env::current_dir()?;
    let script = if let Some(path) = matches.get_one::<String>("build_script") {
        base.join(path)
    } else {
        base.join("../node_modules/react-native/scripts/react-native-xcode.sh")
    }
    .canonicalize()?;

    info!(
        "Issuing a command for Organization: {} Project: {}",
        org, project
    );

    // if we allow fetching and we detect a simulator run, then we need to switch
    // to simulator mode.
    let fetch_url;
    if_chain! {
        if matches.get_flag("allow_fetch");
        if let Ok(val) = env::var("PLATFORM_NAME");
        if val.ends_with("simulator");
        then {
            let url = matches.get_one::<String>("fetch_from").map(String::as_str).unwrap_or("http://127.0.0.1:8081/");
            info!("Fetching sourcemaps from {}", url);
            fetch_url = Some(url);
        } else {
            info!("Using react-native build script at {}", base.display());
            fetch_url = None;
        }
    }

    // in case we are in debug mode we directly dispatch to the script
    // and exit out early.
    if !should_wrap && fetch_url.is_none() {
        info!("Running in debug mode, skipping script wrapping.");
        let rv = process::Command::new(&script).spawn()?.wait()?;
        propagate_exit_status(rv);
        return Ok(());
    }

    let report_file = TempFile::create()?;
    let node = find_node();
    info!("Using node interpreter '{}'", &node);
    let hermesc = find_hermesc();
    info!("Using hermesc interpreter '{}'", &hermesc);

    MayDetach::wrap("React native symbol handling", |md| {
        let bundle_path;
        let sourcemap_path;
        let bundle_url;
        let sourcemap_url;
        let bundle_file;
        let sourcemap_file;

        // If we have a fetch URL we need to fetch them from there now.  In that
        // case we do indeed fetch it right from the running packager and then
        // store it in temporary files for later consumption.
        if let Some(url) = fetch_url {
            if !matches.get_flag("force_foreground") {
                md.may_detach()?;
            }
            let api = Api::current();
            let url = url.trim_end_matches('/');
            bundle_file = TempFile::create()?;
            bundle_path = bundle_file.path().to_path_buf();
            bundle_url = "~/index.ios.bundle".to_string();
            sourcemap_file = TempFile::create()?;
            sourcemap_path = sourcemap_file.path().to_path_buf();
            sourcemap_url = "~/index.ios.map".to_string();

            // wait up to 10 seconds for the server to be up.
            if !api.wait_until_available(url, Duration::seconds(10))? {
                bail!("Error: react-native packager did not respond in time");
            }

            api.download(
                &format!("{url}/index.ios.bundle?platform=ios&dev=true"),
                &mut bundle_file.open()?,
            )?;
            api.download(
                &format!("{url}/index.ios.map?platform=ios&dev=true"),
                &mut sourcemap_file.open()?,
            )?;

        // This is the case where we need to hook into the release process to
        // collect sourcemaps when they are generated.
        //
        // this invokes via an indirection of sentry-cli our wrap_call() below.
        // What is happening behind the scenes is that we switch out NODE_BINARY
        // for ourselves which is what the react-native build script normally
        // invokes.  Because we export __SENTRY_RN_WRAP_XCODE_CALL=1, the main
        // sentry-cli script will invoke our wrap_call() function below.
        //
        // That will then attempt to figure out that a react-native bundle is
        // happening to the build script, parse out the arguments, add additional
        // arguments if needed and then report the parsed arguments to a temporary
        // JSON file we load back below.
        //
        // We do the same for Hermes Compiler to retrieve the bundle file and
        // the same for the combine source maps for the final Hermes source map.
        //
        // With that we we then have all the information we need to invoke the
        // upload process.
        } else {
            let mut command = process::Command::new(&script);
            command
                .env("NODE_BINARY", env::current_exe()?.to_str().unwrap())
                .env("SENTRY_RN_REAL_NODE_BINARY", &node)
                .env(
                    "SENTRY_RN_SOURCEMAP_REPORT",
                    report_file.path().to_str().unwrap(),
                )
                .env("__SENTRY_RN_WRAP_XCODE_CALL", "1");

            if is_hermes_enabled(&hermesc) {
                command
                    .env("HERMES_CLI_PATH", env::current_exe()?.to_str().unwrap())
                    .env("SENTRY_RN_REAL_HERMES_CLI_PATH", &hermesc);
            }

            let rv = command.spawn()?.wait()?;
            propagate_exit_status(rv);

            if !matches.get_flag("force_foreground") {
                md.may_detach()?;
            }
            let mut f = fs::File::open(report_file.path())?;
            let report: SourceMapReport = serde_json::from_reader(&mut f).unwrap_or_else(|_| {
                let format_err = format!(
                    "File {} doesn't contain a valid JSON data.",
                    report_file.path().display()
                );
                panic!("{}", format_err);
            });
            let (Some(packager_bundle_path), Some(packager_sourcemap_path)) =
                (report.packager_bundle_path, report.packager_sourcemap_path)
            else {
                println!("Warning: build produced no packager sourcemaps.");
                return Ok(());
            };

            // If Hermes emitted source map we have to use it
            if let (Some(hermes_bundle_path), Some(hermes_sourcemap_path)) =
                (report.hermes_bundle_path, report.hermes_sourcemap_path)
            {
                bundle_path = hermes_bundle_path.clone();
                sourcemap_path = hermes_sourcemap_path.clone();
                println!("Using Hermes bundle and combined source map.");

            // If Hermes emitted only bundle or Hermes was disabled use packager bundle and source map
            } else {
                bundle_path = packager_bundle_path;
                sourcemap_path = packager_sourcemap_path;
                println!("Using React Native Packager bundle and source map.");
            }
            bundle_url = format!("~/{}", bundle_path.file_name().unwrap().to_string_lossy());
            sourcemap_url = format!(
                "~/{}",
                sourcemap_path.file_name().unwrap().to_string_lossy()
            );
        }

        // now that we have all the data, we can now process and upload the
        // sourcemaps.
        println!("Processing react-native sourcemaps for Sentry upload.");
        info!("  bundle path: {}", bundle_path.display());
        info!("  sourcemap path: {}", sourcemap_path.display());

        let mut processor = SourceMapProcessor::new();
        processor.add(&bundle_url, ReleaseFileSearch::collect_file(bundle_path)?)?;
        processor.add(
            &sourcemap_url,
            ReleaseFileSearch::collect_file(sourcemap_path)?,
        )?;
        processor.rewrite(&[base.parent().unwrap().to_str().unwrap()])?;
        processor.add_sourcemap_references()?;
        processor.add_debug_id_references()?;

        let api = Api::current();
        let chunk_upload_options = api.authenticated()?.get_chunk_upload_options(&org)?;

        let dist_from_env = env::var("SENTRY_DIST");
        let release_from_env = env::var("SENTRY_RELEASE");

        let wait_for_secs = matches.get_one::<u64>("wait_for").copied();
        let wait = matches.get_flag("wait") || wait_for_secs.is_some();
        let max_wait = wait_for_secs.map_or(DEFAULT_MAX_WAIT, std::time::Duration::from_secs);

        if dist_from_env.is_err()
            && release_from_env.is_err()
            && matches.get_flag("no_auto_release")
        {
            processor.upload(&UploadContext {
                org: &org,
                project: Some(&project),
                release: None,
                dist: None,
                note: None,
                wait,
                max_wait,
                dedupe: false,
                chunk_upload_options: chunk_upload_options.as_ref(),
            })?;
        } else {
            let (dist, release_name) = match (&dist_from_env, &release_from_env) {
                (Err(_), Err(_)) => {
                    // Neither environment variable is present, attempt to parse Info.plist
                    info!("Parsing Info.plist");
                    match InfoPlist::discover_from_env() {
                        Ok(Some(plist)) => {
                            // Successfully discovered and parsed Info.plist
                            let dist_string = plist.build().to_string();
                            let release_string = format!(
                                "{}@{}+{}",
                                plist.bundle_id(),
                                plist.version(),
                                dist_string
                            );
                            info!("Parse result from Info.plist: {:?}", &plist);
                            (Some(dist_string), Some(release_string))
                        }
                        _ => {
                            bail!("Info.plist was not found or an parsing error occurred");
                        }
                    }
                }
                // At least one environment variable is present, use the values from the environment
                _ => (dist_from_env.ok(), release_from_env.ok()),
            };

            match matches.get_many::<String>("dist") {
                None => {
                    processor.upload(&UploadContext {
                        org: &org,
                        project: Some(&project),
                        release: release_name.as_deref(),
                        dist: dist.as_deref(),
                        note: None,
                        wait,
                        max_wait,
                        dedupe: false,
                        chunk_upload_options: chunk_upload_options.as_ref(),
                    })?;
                }
                Some(dists) => {
                    for dist in dists {
                        processor.upload(&UploadContext {
                            org: &org,
                            project: Some(&project),
                            release: release_name.as_deref(),
                            dist: Some(dist),
                            note: None,
                            wait,
                            max_wait,
                            dedupe: false,
                            chunk_upload_options: chunk_upload_options.as_ref(),
                        })?;
                    }
                }
            }
        }

        Ok(())
    })
}

pub fn wrap_call() -> Result<()> {
    let mut execute_hermes_compiler = false;
    let mut should_copy_debug_id = false;
    let mut args: Vec<_> = env::args().skip(1).collect();
    let mut bundle_path = None;
    let mut sourcemap_path = None;
    let bundle_command = env::var("SENTRY_RN_BUNDLE_COMMAND");
    let compose_source_maps_path = env::var("COMPOSE_SOURCEMAP_PATH");
    let no_debug_id = env::var("SENTRY_RN_NO_DEBUG_ID").unwrap_or("0".to_string()) == "1";

    let report_file_path = env::var("SENTRY_RN_SOURCEMAP_REPORT").unwrap();
    let mut sourcemap_report: SourceMapReport = if std::path::Path::new(&report_file_path).exists()
    {
        let mut f = fs::File::open(report_file_path.clone())?;
        serde_json::from_reader(&mut f).unwrap_or_else(|_| SourceMapReport::default())
    } else {
        SourceMapReport::default()
    };

    // bundle and ram-bundle are React Native CLI commands
    // export:embed is an Expo CLI command (drop in replacement for bundle)
    // if bundle_command is set, ignore the default values
    if args.len() > 1
        && ((bundle_command.is_err()
            && (args[1] == "bundle" || args[1] == "ram-bundle" || args[1] == "export:embed"))
            || (bundle_command.is_ok() && args[1] == bundle_command.unwrap()))
    {
        let mut iter = args.iter().fuse();
        while let Some(item) = iter.next() {
            if item == "--sourcemap-output" {
                sourcemap_path = iter.next().cloned();
            } else if let Some(rest) = item.strip_prefix("--sourcemap-output=") {
                sourcemap_path = Some(rest.to_string());
            } else if item == "--bundle-output" {
                bundle_path = iter.next().cloned();
            } else if let Some(rest) = item.strip_prefix("--bundle-output=") {
                bundle_path = Some(rest.to_string());
            }
        }

        if sourcemap_path.is_none() && bundle_path.is_some() {
            let mut path = env::temp_dir();
            let mut map_path = PathBuf::from(bundle_path.clone().unwrap());
            map_path.set_extension("jsbundle.map");
            path.push(map_path.file_name().unwrap());
            sourcemap_report.packager_sourcemap_path = Some(PathBuf::from(&path));
            args.push("--sourcemap-output".into());
            args.push(path.into_os_string().into_string().unwrap());
        } else if let Some(path) = sourcemap_path {
            sourcemap_report.packager_sourcemap_path = Some(PathBuf::from(path));
        }

        sourcemap_report.packager_bundle_path = bundle_path.map(PathBuf::from);

    // Hermes Compiler
    // -emit-binary doesn't have to be first in order but all
    // supported RN 0.65 to 0.72 have it as first argument
    // and users can't change it
    } else if args.len() > 1 && args[0] == "-emit-binary" {
        execute_hermes_compiler = true;
        let mut iter = args.iter().fuse();
        while let Some(item) = iter.next() {
            if item == "-out" {
                bundle_path = iter.next().cloned();
            }
        }

        sourcemap_report.hermes_bundle_path = bundle_path.map(PathBuf::from);

    // Combine Source Maps Script
    // We don't check -output-source-map the previous hermesc
    // because we need the final source map not the intermediate hermes only one
    // combine source maps script is execute only if hermes emitted source maps
    // if not packages bundle and sourcemap have to be used for symbolication
    //
    // The compose script can be user defined so we have to check for that
    } else if args.len() > 1
        && (args[0].ends_with("compose-source-maps.js")
            || (compose_source_maps_path.is_ok() && args[0] == compose_source_maps_path.unwrap()))
    {
        let mut iter = args.iter().fuse();
        while let Some(item) = iter.next() {
            if item == "-o" {
                sourcemap_path = iter.next().cloned();
            }
        }

        sourcemap_report.hermes_sourcemap_path = sourcemap_path.map(PathBuf::from);
        should_copy_debug_id = true;
    }

    let executable = if execute_hermes_compiler {
        env::var("SENTRY_RN_REAL_HERMES_CLI_PATH").unwrap()
    } else {
        env::var("SENTRY_RN_REAL_NODE_BINARY").unwrap()
    };
    let rv = process::Command::new(executable)
        .args(args)
        .spawn()?
        .wait()?;
    propagate_exit_status(rv);

    if !no_debug_id && should_copy_debug_id {
        // Copy debug id to the combined source map
        // We have to copy the debug id from the packager source map
        // because the combine source map doesn't copy it over
        // We have to do it while pretending being the script because of the clean up afterwards
        if let Some(ref packager_sourcemap_path) = sourcemap_report.packager_sourcemap_path {
            let mut packager_sourcemap_file = fs::File::open(packager_sourcemap_path)?;
            let packager_sourcemap_result: Result<HashMap<String, Value>, serde_json::Error> =
                serde_json::from_reader(&mut packager_sourcemap_file);

            let hermes_sourcemap_path = sourcemap_report.hermes_sourcemap_path.as_ref().unwrap();
            let mut hermes_sourcemap_file = fs::File::open(hermes_sourcemap_path)?;
            let hermes_sourcemap_result: Result<HashMap<String, Value>, serde_json::Error> =
                serde_json::from_reader(&mut hermes_sourcemap_file);

            if packager_sourcemap_result.is_err() {
                println!(
                    "React Native Packager source map {} doesn't contain a valid JSON data, skipping copy of debug id to Hermes combined source map.",
                    packager_sourcemap_path.as_path().display(),
                );
            }

            if hermes_sourcemap_result.is_err() {
                println!(
                    "Hermes combined source map {} doesn't contain a valid JSON data, skipping copy of debug id to Hermes combined source map.",
                    hermes_sourcemap_path.as_path().display(),
                );
            }

            if let (Ok(packager_sourcemap), Ok(mut hermes_sourcemap)) =
                (packager_sourcemap_result, hermes_sourcemap_result)
            {
                if !hermes_sourcemap.contains_key("debugId")
                    && !hermes_sourcemap.contains_key("debug_id")
                {
                    if let Some(debug_id) = packager_sourcemap
                        .get("debugId")
                        .or_else(|| packager_sourcemap.get("debug_id"))
                    {
                        hermes_sourcemap.insert("debugId".to_string(), debug_id.clone());
                        hermes_sourcemap.insert("debug_id".to_string(), debug_id.clone());

                        hermes_sourcemap_file = fs::File::create(hermes_sourcemap_path)?;
                        serde_json::to_writer(&mut hermes_sourcemap_file, &hermes_sourcemap)?;
                    } else {
                        println!("No debug id found in packager source map, skipping copy to Hermes combined source map.");
                    }
                } else {
                    println!("Hermes combined source map already contains a debug id, skipping copy from packager source map.");
                }
            }
        } else {
            println!("No packager source map found in source map report, skipping copy of debug id to Hermes combined source map.");
        }
    }

    let mut f = fs::File::create(&report_file_path)?;
    serde_json::to_writer(&mut f, &sourcemap_report)?;

    Ok(())
}
