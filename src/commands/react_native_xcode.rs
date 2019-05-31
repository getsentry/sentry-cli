//! Implements a command for uploading react-native projects.
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use chrono::Duration;
use clap::{App, Arg, ArgMatches};
use failure::{bail, Error};
use if_chain::if_chain;
use log::info;
use serde::{Deserialize, Serialize};

use crate::api::{Api, NewRelease};
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::fs::TempFile;
use crate::utils::sourcemaps::{SourceMapProcessor, UploadContext};
use crate::utils::system::propagate_exit_status;
use crate::utils::xcode::{InfoPlist, MayDetach};

#[derive(Serialize, Deserialize, Default, Debug)]
struct SourceMapReport {
    bundle_path: Option<PathBuf>,
    sourcemap_path: Option<PathBuf>,
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Upload react-native projects in a Xcode build step.")
        .org_project_args()
        // legacy parameter
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .hidden(true),
        )
        .arg(Arg::with_name("force").long("force").short("f").help(
            "Force the script to run, even in debug configuration.{n}This rarely \
             does what you want because the default build script does not actually \
             produce any information that the sentry build tool could pick up on.",
        ))
        .arg(Arg::with_name("allow_fetch").long("allow-fetch").help(
            "Enable sourcemap fetching from the packager.{n}If this is enabled \
             the react native packager needs to run and sourcemaps are downloade \
             from it if the simulator platform is detected.",
        ))
        .arg(
            Arg::with_name("fetch_from")
                .long("fetch-from")
                .value_name("URL")
                .help(
                    "Set the URL to fetch sourcemaps from.{n}\
                     The default is http://127.0.0.1:8081/, where the react-native \
                     packager runs by default.",
                ),
        )
        .arg(
            Arg::with_name("force_foreground")
                .long("force-foreground")
                .help(
                    "Wait for the process to finish.{n}\
                     By default part of the build process will when triggered from Xcode \
                     detach and continue in the background.  When an error happens, \
                     a dialog is shown.  If this parameter is passed, Xcode will wait \
                     for the process to finish before the build finishes and output \
                     will be shown in the Xcode build output.",
                ),
        )
        .arg(
            Arg::with_name("build_script")
                .value_name("BUILD_SCRIPT")
                .index(1)
                .help(
                    "Optional path to the build script.{n}\
                     This is the path to the `react-native-xcode.sh` script you want \
                     to use.  By default the bundled build script is used.",
                ),
        )
        .arg(
            Arg::with_name("args")
                .value_name("ARGS")
                .multiple(true)
                .last(true)
                .help("Optional arguments to pass to the build script."),
        )
        .arg(
            Arg::with_name("wait")
                .long("wait")
                .help("Wait for the server to fully process uploaded files."),
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

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let api = Api::current();
    let should_wrap = matches.is_present("force")
        || match env::var("CONFIGURATION") {
            Ok(config) => &config != "Debug",
            Err(_) => bail!("Need to run this from Xcode"),
        };
    let base = env::current_dir()?;
    let script = if let Some(path) = matches.value_of("build_script") {
        base.join(path)
    } else {
        base.join("../node_modules/react-native/scripts/react-native-xcode.sh")
    }
    .canonicalize()?;

    // if we allow fetching and we detect a simulator run, then we need to switch
    // to simulator mode.
    let fetch_url;
    if_chain! {
        if matches.is_present("allow_fetch");
        if let Ok(val) = env::var("PLATFORM_NAME");
        if val.ends_with("simulator");
        then {
            let url = matches.value_of("fetch_from").unwrap_or("http://127.0.0.1:8081/");
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

    info!("Parsing Info.plist");
    let plist = match InfoPlist::discover_from_env()? {
        Some(plist) => plist,
        None => bail!("Could not find info.plist"),
    };
    info!("Parse result from Info.plist: {:?}", &plist);
    let report_file = TempFile::create()?;
    let node = find_node();
    info!("Using node interpreter '{}'", &node);

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
            if !matches.is_present("force_foreground") {
                md.may_detach()?;
            }
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
                &format!("{}/index.ios.bundle?platform=ios&dev=true", url),
                &mut bundle_file.open()?,
            )?;
            api.download(
                &format!("{}/index.ios.map?platform=ios&dev=true", url),
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
        // With that we we then have all the information we need to invoke the
        // upload process.
        } else {
            let rv = process::Command::new(&script)
                .env("NODE_BINARY", env::current_exe()?.to_str().unwrap())
                .env("SENTRY_RN_REAL_NODE_BINARY", &node)
                .env(
                    "SENTRY_RN_SOURCEMAP_REPORT",
                    report_file.path().to_str().unwrap(),
                )
                .env("__SENTRY_RN_WRAP_XCODE_CALL", "1")
                .spawn()?
                .wait()?;
            propagate_exit_status(rv);

            if !matches.is_present("force_foreground") {
                md.may_detach()?;
            }
            let mut f = fs::File::open(report_file.path())?;
            let report: SourceMapReport = serde_json::from_reader(&mut f)?;
            if report.bundle_path.is_none() || report.sourcemap_path.is_none() {
                println!("Warning: build produced no sourcemaps.");
                return Ok(());
            }

            bundle_path = report.bundle_path.unwrap();
            bundle_url = format!("~/{}", bundle_path.file_name().unwrap().to_string_lossy());
            sourcemap_path = report.sourcemap_path.unwrap();
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
        processor.add(&bundle_url, &bundle_path)?;
        processor.add(&sourcemap_url, &sourcemap_path)?;
        processor.rewrite(&[base.parent().unwrap().to_str().unwrap()])?;
        processor.add_sourcemap_references()?;

        let release = api.new_release(
            &org,
            &NewRelease {
                version: format!("{}-{}", plist.bundle_id(), plist.version()),
                projects: vec![project.to_string()],
                ..Default::default()
            },
        )?;

        processor.upload(&UploadContext {
            org: &org,
            project: Some(&project),
            release: &release.version,
            dist: Some(&plist.build()),
            wait: matches.is_present("wait"),
        })?;

        Ok(())
    })
}

pub fn wrap_call() -> Result<(), Error> {
    let mut args: Vec<_> = env::args().skip(1).collect();
    let mut bundle_path = None;
    let mut sourcemap_path = None;

    if args.len() > 1 && (args[1] == "bundle" || args[1] == "ram-bundle") {
        let mut iter = args.iter().fuse();
        while let Some(item) = iter.next() {
            if item == "--sourcemap-output" {
                sourcemap_path = iter.next().cloned();
            } else if item.starts_with("--sourcemap-output=") {
                sourcemap_path = Some(item[19..].to_string());
            } else if item == "--bundle-output" {
                bundle_path = iter.next().cloned();
            } else if item.starts_with("--bundle-output=") {
                bundle_path = Some(item[16..].to_string());
            }
        }
    }

    let mut sourcemap_report = SourceMapReport::default();

    if sourcemap_path.is_none() && bundle_path.is_some() {
        let path = format!("{}.map", &bundle_path.as_ref().unwrap());
        sourcemap_report.sourcemap_path = Some(PathBuf::from(&path));
        args.push("--sourcemap-output".into());
        args.push(path);
    } else if let Some(path) = sourcemap_path {
        sourcemap_report.sourcemap_path = Some(PathBuf::from(path));
    }

    sourcemap_report.bundle_path = bundle_path.map(PathBuf::from);

    let rv = process::Command::new(env::var("SENTRY_RN_REAL_NODE_BINARY").unwrap())
        .args(&args)
        .spawn()?
        .wait()?;
    propagate_exit_status(rv);

    let mut f = fs::File::create(env::var("SENTRY_RN_SOURCEMAP_REPORT").unwrap())?;
    serde_json::to_writer(&mut f, &sourcemap_report)?;

    Ok(())
}
