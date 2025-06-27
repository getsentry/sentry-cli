#![expect(clippy::unwrap_used, reason = "deprecated command")]

use std::cmp::min;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{format_err, Result};
use clap::{builder::ArgPredicate, Arg, ArgAction, ArgMatches, Command};
use lazy_static::lazy_static;
use regex::Regex;
use sentry::protocol::{Event, Exception, Frame, Stacktrace, User, Value};
use uuid::Uuid;

use crate::commands::send_event;
use crate::config::Config;
use crate::utils::event::{attach_logfile, get_sdk_info};
use crate::utils::releases::detect_release_name;

const BASH_SCRIPT: &str = include_str!("../bashsupport.sh");
lazy_static! {
    static ref FRAME_RE: Regex = Regex::new(r"^(.*?):(.*):(\d+)$").unwrap();
}

pub fn make_command(command: Command) -> Command {
    command
        .about("Prints out a bash script that does error handling.")
        // Legacy command, left hidden for backward compatibility
        .hide(true)
        .arg(
            Arg::new("no_exit")
                .long("no-exit")
                .action(ArgAction::SetTrue)
                .help("Do not turn on -e (exit immediately) flag automatically"),
        )
        .arg(
            Arg::new("no_environ")
                .long("no-environ")
                .action(ArgAction::SetTrue)
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
                .action(ArgAction::SetTrue)
                .requires_ifs([
                    (ArgPredicate::IsPresent, "traceback"),
                    (ArgPredicate::IsPresent, "log"),
                ])
                .hide(true),
        )
        .arg(
            Arg::new("traceback")
                .long("traceback")
                .value_name("PATH")
                .hide(true),
        )
        .arg(
            Arg::new("tags")
                .value_name("KEY:VALUE")
                .long("tag")
                .action(ArgAction::Append)
                .help("Add tags (key:value) to the event."),
        )
        .arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .action(ArgAction::Set)
                .help("Define release version for the event."),
        )
        .arg(Arg::new("log").long("log").value_name("PATH").hide(true))
}

fn send_event(
    traceback: &str,
    logfile: &str,
    tags: &[&String],
    release: Option<String>,
    environ: bool,
) -> Result<()> {
    let config = Config::current();

    let mut event = Event {
        environment: config.get_environment().map(Into::into),
        release: release.or(detect_release_name().ok()).map(Into::into),
        sdk: Some(get_sdk_info()),
        user: whoami::fallible::username().ok().map(|n| User {
            username: Some(n),
            ip_address: Some(Default::default()),
            ..Default::default()
        }),
        ..Event::default()
    };

    for tag in tags {
        let mut split = tag.splitn(2, ':');
        let key = split.next().ok_or_else(|| format_err!("missing tag key"))?;
        let value = split
            .next()
            .ok_or_else(|| format_err!("missing tag value"))?;
        event.tags.insert(key.into(), value.into());
    }

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

    let id = send_event::send_raw_event(event)?;
    println!("{id}");

    Ok(())
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let release = Config::current().get_release(matches).ok();

    let tags: Vec<_> = matches
        .get_many::<String>("tags")
        .map(|v| v.collect())
        .unwrap_or_default();

    if matches.get_flag("send_event") {
        return send_event(
            matches.get_one::<String>("traceback").unwrap(),
            matches.get_one::<String>("log").unwrap(),
            &tags,
            release,
            !matches.get_flag("no_environ"),
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
        " ___SENTRY_TAGS___",
        &tags
            .iter()
            .map(|tag| format!(" --tag \"{tag}\""))
            .collect::<Vec<_>>()
            .join(""),
    );

    script = match release {
        Some(release) => script.replace(
            " ___SENTRY_RELEASE___",
            format!(" --release \"{release}\"").as_str(),
        ),
        None => script.replace(" ___SENTRY_RELEASE___", ""),
    };

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

    if matches.get_flag("no_environ") {
        script = script.replace("___SENTRY_NO_ENVIRON___", "--no-environ");
    } else {
        script = script.replace("___SENTRY_NO_ENVIRON___", "");
    }

    if !matches.get_flag("no_exit") {
        script.insert_str(0, "set -e\n\n");
    }
    println!("{script}");
    Ok(())
}
