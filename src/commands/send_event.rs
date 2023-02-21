use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{format_err, Result};
use chrono::{DateTime, Utc};
use clap::{Arg, ArgAction, ArgMatches, Command};
use glob::{glob_with, MatchOptions};
use itertools::Itertools;
use log::{debug, warn};
use sentry::protocol::{Event, Level, LogEntry, User};
use sentry::types::{Dsn, Uuid};
use serde_json::Value;
use username::get_user_name;

use crate::config::Config;
use crate::utils::args::{get_timestamp, validate_distribution};
use crate::utils::event::{attach_logfile, get_sdk_info, with_sentry_client};
use crate::utils::releases::detect_release_name;

pub fn make_command(command: Command) -> Command {
    command.about("Send a manual event to Sentry.")
        .long_about(
            "Send a manual event to Sentry.{n}{n}\
             This command will validate input parameters and attempt to send an event to \
             Sentry. Due to network errors, rate limits or sampling the event is not guaranteed to \
             actually arrive. Check debug output for transmission errors by passing --log-level=\
             debug or setting `SENTRY_LOG_LEVEL=debug`.",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(false)
                .help("The path or glob to the file(s) in JSON format to send as event(s). When provided, all other arguments are ignored."),
        )
        .arg(
            Arg::new("level")
                .value_name("LEVEL")
                .long("level")
                .short('l')
                .help("Optional event severity/log level. (debug|info|warning|error|fatal) [defaults to 'error']"),
        )
        .arg(Arg::new("timestamp")
                 .long("timestamp")
                 .value_parser(get_timestamp)
                 .value_name("TIMESTAMP")
                 .help("Optional event timestamp in one of supported formats: unix timestamp, RFC2822 or RFC3339."))
        .arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .short('r')
                .help("Optional identifier of the release."),
        )
        .arg(
            Arg::new("dist")
                .value_name("DISTRIBUTION")
                .long("dist")
                .short('d')
                .value_parser(validate_distribution)
                .help("Set the distribution."),
        )
        .arg(
            Arg::new("environment")
                .value_name("ENVIRONMENT")
                .long("env")
                .short('E')
                .help("Send with a specific environment."),
        )
        .arg(
            Arg::new("no_environ")
                .long("no-environ")
                .help("Do not send environment variables along"),
        )
        .arg(
            Arg::new("message")
                .value_name("MESSAGE")
                .long("message")
                .short('m')
                .action(ArgAction::Append)
                .help("The event message."),
        )
        .arg(
            Arg::new("message_args")
                .value_name("MESSAGE_ARG")
                .long("message-arg")
                .short('a')
                .action(ArgAction::Append)
                .help("Arguments for the event message."),
        )
        .arg(
            Arg::new("platform")
                .value_name("PLATFORM")
                .long("platform")
                .short('p')
                .help("Override the default 'other' platform specifier."),
        )
        .arg(
            Arg::new("tags")
                .value_name("KEY:VALUE")
                .long("tag")
                .short('t')
                .action(ArgAction::Append)
                .help("Add a tag (key:value) to the event."),
        )
        .arg(
            Arg::new("extra")
                .value_name("KEY:VALUE")
                .long("extra")
                .short('e')
                .action(ArgAction::Append)
                .help("Add extra information (key:value) to the event."),
        )
        .arg(
            Arg::new("user_data")
                .value_name("KEY:VALUE")
                .long("user")
                .short('u')
                .action(ArgAction::Append)
                .help(
                    "Add user information (key:value) to the event. \
                     [eg: id:42, username:foo]",
                ),
        )
        .arg(
            Arg::new("fingerprint")
                .value_name("FINGERPRINT")
                .long("fingerprint")
                .short('f')
                .action(ArgAction::Append)
                .help("Change the fingerprint of the event."),
        )
        .arg(
            Arg::new("logfile")
                .value_name("PATH")
                .long("logfile")
                .help("Send a logfile as breadcrumbs with the event (last 100 records)"),
        )
        .arg(
            Arg::new("with_categories")
                .long("with-categories")
                .help("Parses off a leading category for breadcrumbs from the logfile")
                .long_help(
                    "When logfile is provided, this flag will try to assign correct level \
                    to extracted log breadcrumbs. It uses standard log format of \"category: message\". \
                    eg. \"INFO: Something broke\" will be parsed as a breadcrumb \
                    \"{\"level\": \"info\", \"message\": \"Something broke\"}\"")
        )
}

fn send_raw_event(event: Event<'static>, dsn: Dsn) -> Uuid {
    debug!("{:?}", event);
    with_sentry_client(dsn, |c| c.capture_event(event, None))
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let dsn = config.get_dsn()?;

    if let Some(path) = matches.get_one::<String>("path") {
        let collected_paths: Vec<PathBuf> = glob_with(path, MatchOptions::new())
            .unwrap()
            .flatten()
            .collect();

        if collected_paths.is_empty() {
            warn!("Did not match any .json files for pattern: {}", path);
            return Ok(());
        }

        for path in collected_paths {
            let p = path.as_path();
            let file = File::open(p)?;
            let reader = BufReader::new(file);
            let event: Event = serde_json::from_reader(reader)?;
            let id = send_raw_event(event, dsn.clone());
            println!("Event from file {} dispatched: {}", p.display(), id);
        }

        return Ok(());
    }

    let mut event = Event {
        sdk: Some(get_sdk_info()),
        level: matches
            .get_one::<String>("level")
            .and_then(|l| l.parse().ok())
            .unwrap_or(Level::Error),
        release: matches
            .get_one::<String>("release")
            .map(|s| Cow::Owned(s.clone()))
            .or_else(|| detect_release_name().ok().map(Cow::from)),
        dist: matches
            .get_one::<String>("dist")
            .map(|s| Cow::Owned(s.clone())),
        platform: matches
            .get_one::<String>("platform")
            .map(|s| Cow::Owned(s.clone()))
            .unwrap_or(Cow::from("other")),
        environment: matches
            .get_one::<String>("environment")
            .map(|s| Cow::Owned(s.clone())),
        logentry: matches
            .get_many::<String>("message")
            .map(|mut lines| LogEntry {
                message: lines.join("\n"),
                params: matches
                    .get_many::<String>("message_args")
                    .map(|args| args.map(|x| x.as_str().into()).collect())
                    .unwrap_or_default(),
            }),
        ..Event::default()
    };

    if let Some(timestamp) = matches.get_one::<DateTime<Utc>>("timestamp") {
        event.timestamp = SystemTime::from(*timestamp);
    }

    for tag in matches.get_many::<String>("tags").unwrap_or_default() {
        let mut split = tag.splitn(2, ':');
        let key = split.next().ok_or_else(|| format_err!("missing tag key"))?;
        let value = split
            .next()
            .ok_or_else(|| format_err!("missing tag value"))?;
        event.tags.insert(key.into(), value.into());
    }

    if !matches.contains_id("no_environ") {
        event.extra.insert(
            "environ".into(),
            Value::Object(env::vars().map(|(k, v)| (k, Value::String(v))).collect()),
        );
    }

    for pair in matches.get_many::<String>("extra").unwrap_or_default() {
        let mut split = pair.splitn(2, ':');
        let key = split
            .next()
            .ok_or_else(|| format_err!("missing extra key"))?;
        let value = split
            .next()
            .ok_or_else(|| format_err!("missing extra value"))?;
        event.extra.insert(key.into(), Value::String(value.into()));
    }

    if let Some(user_data) = matches.get_many::<String>("user_data") {
        let mut user = User::default();
        for pair in user_data {
            let mut split = pair.splitn(2, ':');
            let key = split
                .next()
                .ok_or_else(|| format_err!("missing user key"))?;
            let value = split
                .next()
                .ok_or_else(|| format_err!("missing user value"))?;

            match key {
                "id" => user.id = Some(value.into()),
                "email" => user.email = Some(value.into()),
                "ip_address" => user.ip_address = Some(value.parse()?),
                "username" => user.username = Some(value.into()),
                _ => {
                    user.other.insert(key.into(), value.into());
                }
            };
        }

        user.ip_address.get_or_insert(Default::default());
        event.user = Some(user);
    } else {
        event.user = get_user_name().ok().map(|n| User {
            username: Some(n),
            ip_address: Some(Default::default()),
            ..Default::default()
        });
    }

    if let Some(fingerprint) = matches.get_many::<String>("fingerprint") {
        event.fingerprint = fingerprint
            .map(|x| x.to_string().into())
            .collect::<Vec<_>>()
            .into();
    }

    if let Some(logfile) = matches.get_one::<String>("logfile") {
        attach_logfile(&mut event, logfile, matches.contains_id("with_categories"))?;
    }

    let id = send_raw_event(event, dsn);
    println!("Event dispatched: {id}");

    Ok(())
}
