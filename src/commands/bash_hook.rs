//! Implements a command for showing infos from Sentry.
use std::fs;
use std::io::{BufRead, BufReader};
use std::env;
use std::cmp::min;
use std::path::Path;
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use uuid::{Uuid, UuidVersion};
use regex::Regex;

use api::Api;
use config::Config;
use errors::Result;
use event::{Event, Exception, SingleException, Frame, Stacktrace};

const BASH_SCRIPT: &'static str = include_str!("../bashsupport.sh");
lazy_static! {
    static ref FRAME_RE: Regex = Regex::new(
        r#"^(.*?):(.*):(\d+)$"#).unwrap();
}


pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Prints out a bash script that does error handling.")
        .arg(Arg::with_name("no_exit")
             .long("no-exit")
             .help("Do not turn on -e (exit immediately) flag automatically"))
        .arg(Arg::with_name("send_event")
            .long("send-event")
            .requires_all(&["traceback", "log"])
            .hidden(true))
        .arg(Arg::with_name("traceback")
            .long("traceback")
            .value_name("PATH")
            .hidden(true))
        .arg(Arg::with_name("log")
            .long("log")
            .value_name("PATH")
            .hidden(true))
}

fn send_event(traceback: &str, logfile: &str) -> Result<()> {
    let config = Config::get_current();
    let mut event = Event::new_prefilled()?;
    event.detect_release();

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
                    "_sentry_err_trap" |
                    "_sentry_exit_trap" |
                    "_sentry_traceback" => continue,
                    _ => {}
                }
                frames.push(Frame {
                    filename: cap[2].to_string(),
                    abs_path: Path::new(&cap[2])
                        .canonicalize().map(|x| x.display().to_string()).ok(),
                    function: cap[1].to_string(),
                    lineno: cap[3].parse().ok(),
                    ..Default::default()
                });
            }
        }
    }

    let mut source_caches = HashMap::new();
    for frame in frames.iter_mut() {
        let lineno = match frame.lineno {
            Some(lineno) => lineno as usize,
            None => continue,
        };
        if !source_caches.contains_key(&frame.filename) {
            if let Ok(f) = fs::File::open(&frame.filename) {
                let lines: Vec<_> = BufReader::new(f)
                    .lines()
                    .map(|x| x.unwrap_or_else(|_| "".to_string()))
                    .collect();
                source_caches.insert(frame.filename.clone(), lines);
            } else {
                source_caches.insert(frame.filename.clone(), vec![]);
            }
        }
        let source = source_caches.get(&frame.filename).unwrap();
        frame.context_line = source.get(
            lineno.saturating_sub(1)).map(|x| x.clone());
        if let Some(slice) = source.get(
            lineno.saturating_sub(5)..lineno.saturating_sub(1)) {
            frame.pre_context = Some(slice.iter().map(|x| x.clone()).collect());
        };
        if let Some(slice) = source.get(
            lineno..min(lineno + 5, source.len())) {
            frame.post_context = Some(slice.iter().map(|x| x.clone()).collect());
        };
    }

    event.attach_logfile(logfile, true)?;

    frames.reverse();
    event.exception = Some(Exception {
        values: vec![SingleException {
            ty: "BashError".into(),
            value: format!("command {} exited with status {}", cmd, exit_code),
            stacktrace: Some(Stacktrace {
                frames: frames,
            }),
        }],
    });

    let dsn = config.get_dsn()?;

    // handle errors here locally so that we do not get the extra "use sentry-cli
    // login" to sign in which would be in appropriate here.
    if let Ok(event_id) = Api::get_current().send_event(&dsn, &event) {
        println!("{}", event_id);
    };

    Ok(())
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<()> {
    if matches.is_present("send_event") {
        return send_event(matches.value_of("traceback").unwrap(),
                          matches.value_of("log").unwrap());
    }

    let path = env::temp_dir();
    let log = path.join(&format!(
        ".sentry-{}.out", Uuid::new(UuidVersion::Random).unwrap().hyphenated().to_string()));
    let traceback = path.join(&format!(
        ".sentry-{}.traceback", Uuid::new(UuidVersion::Random).unwrap().hyphenated().to_string()));
    let mut script = BASH_SCRIPT
        .replace("___SENTRY_TRACEBACK_FILE___", &traceback.display().to_string())
        .replace("___SENTRY_LOG_FILE___", &log.display().to_string())
        .replace("___SENTRY_CLI___", &env::current_exe().unwrap().display().to_string());
    if !matches.is_present("no_exit") {
        script.insert_str(0, "set -e\n\n");
    }
    println!("{}", script);
    Ok(())
}
