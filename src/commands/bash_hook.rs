use std::cmp::min;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use clap::Command;
use clap::{Arg, ArgMatches};
use lazy_static::lazy_static;
use regex::Regex;
use sentry::protocol::{Event, Exception, Frame, Stacktrace, User, Value};
use username::get_user_name;
use uuid::Uuid;

use crate::config::Config;
use crate::utils::event::{attach_logfile, get_sdk_info, with_sentry_client};
use crate::utils::releases::detect_release_name;

const BASH_SCRIPT: &str = include_str!("../bashsupport.sh");
lazy_static! {
    static ref FRAME_RE: Regex = Regex::new(r#"^(.*?):(.*):(\d+)$"#).unwrap();
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Prints out a bash script that does error handling.")
        // Legacy command, left hidden for backward compatibility
        .hide(true)
        .arg(
            Arg::new("no_exit")
                .long("no-exit")
                .help("Do not turn on -e (exit immediately) flag automatically"),
        )
        .arg(
            Arg::new("no_environ")
                .long("no-environ")
                .help("Do not send environment variables along"),
        )
        .arg(
            Arg::new("cli")
                .long("cli")
                .value_name("CMD")
                .help("Explicitly set/override the sentry-cli command"),
        )
        .arg(
            Arg::new("send_event")
                .long("send-event")
                .requires_all(&["traceback", "log"])
                .hide(true),
        )
        .arg(
            Arg::new("traceback")
                .long("traceback")
                .value_name("PATH")
                .hide(true),
        )
        .arg(Arg::new("log").long("log").value_name("PATH").hide(true))
}

fn send_event(traceback: &str, logfile: &str, environ: bool) -> Result<()> {
    let config = Config::current();

    let mut event = Event {
        environment: config.get_environment().map(Into::into),
        release: detect_release_name().ok().map(Into::into),
        sdk: Some(get_sdk_info()),
        user: get_user_name().ok().map(|n| User {
            username: Some(n),
            ip_address: Some(Default::default()),
            ..Default::default()
        }),
        ..Event::default()
    };

    if environ {
        event.extra.insert(
            "environ".into(),
            Value::Object(env::vars().map(|(k, v)| (k, Value::String(v))).collect()),
        );
    }

    let mut cmd = "unknown".to_string();
    let mut exit_code = 1;
    let mut frames = vec![];

    if let Ok(f) = fs::File::open(traceback) {
        let f = BufReader::new(f);
        for line in f.lines() {
            let line = line?;

            // meta info
            if line.starts_with('@') {
                if let Some(rest) = line.strip_prefix("@command:") {
                    cmd = rest.to_string();
                } else if let Some(rest) = line.strip_prefix("@exit_code:") {
                    exit_code = rest.parse().unwrap_or(exit_code);
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
                    filename: Some(cap[2].to_string()),
                    abs_path: Path::new(&cap[2])
                        .canonicalize()
                        .map(|x| x.display().to_string())
                        .ok(),
                    lineno: cap[3].parse().ok(),
                    function: Some(cap[1].to_string()),
                    ..Default::default()
                });
            }
        }
    }

    {
        let mut source_caches = HashMap::new();
        for frame in &mut frames {
            let lineno = match frame.lineno {
                Some(line) => line as usize,
                None => continue,
            };

            let filename = frame.filename.as_deref().expect("frame without location");

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
            let source = &source_caches[filename];
            frame.context_line = source.get(lineno.saturating_sub(1)).cloned();
            if let Some(slice) = source.get(lineno.saturating_sub(5)..lineno.saturating_sub(1)) {
                frame.pre_context = slice.to_vec();
            };
            if let Some(slice) = source.get(lineno..min(lineno + 5, source.len())) {
                frame.post_context = slice.to_vec();
            };
        }
    }

    attach_logfile(&mut event, logfile, true)?;

    event.exception.values.push(Exception {
        ty: "BashError".into(),
        value: Some(format!("command {cmd} exited with status {exit_code}")),
        stacktrace: Some(Stacktrace {
            frames,
            ..Default::default()
        }),
        ..Default::default()
    });

    let id = with_sentry_client(config.get_dsn()?, |c| c.capture_event(event, None));
    println!("{id}");

    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if matches.contains_id("send_event") {
        return send_event(
            matches.get_one::<String>("traceback").unwrap(),
            matches.get_one::<String>("log").unwrap(),
            !matches.contains_id("no_environ"),
        );
    }

    let path = env::temp_dir();
    let log = path.join(format!(".sentry-{}.out", Uuid::new_v4().as_hyphenated()));
    let traceback = path.join(format!(
        ".sentry-{}.traceback",
        Uuid::new_v4().as_hyphenated()
    ));
    let mut script = BASH_SCRIPT
        .replace(
            "___SENTRY_TRACEBACK_FILE___",
            &traceback.display().to_string(),
        )
        .replace("___SENTRY_LOG_FILE___", &log.display().to_string());

    script = script.replace(
        "___SENTRY_CLI___",
        matches
            .get_one::<String>("cli")
            .map_or_else(
                || env::current_exe().unwrap().display().to_string(),
                String::clone,
            )
            .as_str(),
    );

    if matches.contains_id("no_environ") {
        script = script.replace("___SENTRY_NO_ENVIRON___", "--no-environ");
    } else {
        script = script.replace("___SENTRY_NO_ENVIRON___", "");
    }

    if !matches.contains_id("no_exit") {
        script.insert_str(0, "set -e\n\n");
    }
    println!("{script}");
    Ok(())
}
