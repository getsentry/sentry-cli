//! Implements a command for showing infos from Sentry.
use std::cmp::min;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use failure::Error;
use regex::Regex;
use sentry::protocol::{Event, Exception, FileLocation, Frame, Stacktrace, User, Value};
use username::get_user_name;
use uuid::{Uuid, UuidVersion};

use config::Config;
use utils::event::{attach_logfile, get_sdk_info, with_sentry_client};
use utils::releases::detect_release_name;

const BASH_SCRIPT: &'static str = include_str!("../bashsupport.sh");
lazy_static! {
    static ref FRAME_RE: Regex = Regex::new(r#"^(.*?):(.*):(\d+)$"#).unwrap();
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Prints out a bash script that does error handling.")
        .arg(
            Arg::with_name("no_exit")
                .long("no-exit")
                .help("Do not turn on -e (exit immediately) flag automatically"),
        )
        .arg(
            Arg::with_name("send_event")
                .long("send-event")
                .requires_all(&["traceback", "log"])
                .hidden(true),
        )
        .arg(
            Arg::with_name("traceback")
                .long("traceback")
                .value_name("PATH")
                .hidden(true),
        )
        .arg(
            Arg::with_name("log")
                .long("log")
                .value_name("PATH")
                .hidden(true),
        )
}

fn send_event(traceback: &str, logfile: &str) -> Result<(), Error> {
    let config = Config::get_current();
    let mut event = Event::default();

    event.environment = config.get_environment().map(|e| e.into());
    event.release = detect_release_name().ok().map(|r| r.into());
    event.sdk_info = Some(get_sdk_info());
    event.extra.insert(
        "environ".into(),
        Value::Object(env::vars().map(|(k, v)| (k, Value::String(v))).collect()),
    );
    event.user = get_user_name().ok().map(|n| User {
        username: Some(n),
        ip_address: Some(Default::default()),
        ..Default::default()
    });

    let mut cmd = "unknown".to_string();
    let mut exit_code = 1;
    let mut frames = vec![];

    if let Ok(f) = fs::File::open(traceback) {
        let f = BufReader::new(f);
        for line in f.lines() {
            let line = line?;

            // meta info
            if line.starts_with("@") {
                if line.starts_with("@command:") {
                    cmd = line[9..].to_string();
                } else if line.starts_with("@exit_code:") {
                    exit_code = line[11..].parse().unwrap_or(exit_code);
                } else {
                    continue;
                }
            }

            if let Some(cap) = FRAME_RE.captures(&line) {
                match &cap[1] {
                    "_sentry_err_trap" | "_sentry_exit_trap" | "_sentry_traceback" => continue,
                    _ => {}
                }
                frames.push(Frame {
                    location: FileLocation {
                        filename: Some(cap[2].to_string()),
                        abs_path: Path::new(&cap[2])
                            .canonicalize()
                            .map(|x| x.display().to_string())
                            .ok(),
                        line: cap[3].parse().ok(),
                        ..Default::default()
                    },
                    function: Some(cap[1].to_string()),
                    ..Default::default()
                });
            }
        }
    }

    {
        let mut source_caches = HashMap::new();
        for frame in frames.iter_mut() {
            let lineno = match frame.location.line {
                Some(line) => line as usize,
                None => continue,
            };

            let filename = frame
                .location
                .filename
                .as_ref()
                .map(|s| s.as_str())
                .expect("frame without location");

            if !source_caches.contains_key(filename) {
                if let Ok(f) = fs::File::open(filename) {
                    let lines: Vec<_> = BufReader::new(f)
                        .lines()
                        .map(|x| x.unwrap_or_else(|_| "".to_string()))
                        .collect();
                    source_caches.insert(filename, lines);
                } else {
                    source_caches.insert(filename, vec![]);
                }
            }
            let source = source_caches.get(filename).unwrap();
            frame.source.current_line = source.get(lineno.saturating_sub(1)).map(|x| x.clone());
            if let Some(slice) = source.get(lineno.saturating_sub(5)..lineno.saturating_sub(1)) {
                frame.source.pre_lines = slice.iter().map(|x| x.clone()).collect();
            };
            if let Some(slice) = source.get(lineno..min(lineno + 5, source.len())) {
                frame.source.post_lines = slice.iter().map(|x| x.clone()).collect();
            };
        }
    }

    attach_logfile(&mut event, logfile, true)?;

    event.exceptions.push(Exception {
        ty: "BashError".into(),
        value: Some(format!("command {} exited with status {}", cmd, exit_code)),
        stacktrace: Some(Stacktrace {
            frames: frames,
            ..Default::default()
        }),
        ..Default::default()
    });

    if let Some(id) = with_sentry_client(config.get_dsn()?, |c| c.capture_event(event, None)) {
        println!("{}", id);
    }

    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    if matches.is_present("send_event") {
        return send_event(
            matches.value_of("traceback").unwrap(),
            matches.value_of("log").unwrap(),
        );
    }

    let path = env::temp_dir();
    let log = path.join(&format!(
        ".sentry-{}.out",
        Uuid::new(UuidVersion::Random)
            .unwrap()
            .hyphenated()
            .to_string()
    ));
    let traceback = path.join(&format!(
        ".sentry-{}.traceback",
        Uuid::new(UuidVersion::Random)
            .unwrap()
            .hyphenated()
            .to_string()
    ));
    let mut script = BASH_SCRIPT
        .replace(
            "___SENTRY_TRACEBACK_FILE___",
            &traceback.display().to_string(),
        )
        .replace("___SENTRY_LOG_FILE___", &log.display().to_string())
        .replace(
            "___SENTRY_CLI___",
            &env::current_exe().unwrap().display().to_string(),
        );
    if !matches.is_present("no_exit") {
        script.insert_str(0, "set -e\n\n");
    }
    println!("{}", script);
    Ok(())
}
