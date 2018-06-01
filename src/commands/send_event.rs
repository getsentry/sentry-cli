//! Implements a command for sending events to Sentry.
use std::borrow::Cow;

use clap::{App, Arg, ArgMatches};
use failure::{err_msg, Error};
use itertools::Itertools;
use regex::Regex;
use sentry::protocol::{ClientSdkInfo, Event, Level, LogEntry, User};
use sentry::Client;
use serde_json::Value;

use config::Config;
use utils::event::attach_logfile;
use utils::releases::detect_release_name;
use utils::system::QuietExit;

lazy_static! {
    static ref COMPONENT_RE: Regex = Regex::new(r#"^([^:]+): (.*)$"#).unwrap();
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Send a manual event to Sentry.")
        .arg(
            Arg::with_name("level")
                .value_name("LEVEL")
                .long("level")
                .short("l")
                .help("Optional event severity/log level. [defaults to 'error']"),
        )
        .arg(
            Arg::with_name("release")
                .value_name("RELEASE")
                .long("release")
                .short("r")
                .help("Optional identifier of the release."),
        )
        .arg(
            Arg::with_name("dist")
                .value_name("DISTRIBUTION")
                .long("dist")
                .short("d")
                .help("Set the distribution."),
        )
        .arg(
            Arg::with_name("environment")
                .value_name("ENVIRONMENT")
                .long("env")
                .short("E")
                .help("Send with a specific environment."),
        )
        .arg(
            Arg::with_name("no_environ")
                .long("no-environ")
                .help("Do not send environment variables along"),
        )
        .arg(
            Arg::with_name("message")
                .value_name("MESSAGE")
                .long("message")
                .short("m")
                .multiple(true)
                .number_of_values(1)
                .help("The event message."),
        )
        .arg(
            Arg::with_name("message_args")
                .value_name("MESSAGE_ARG")
                .long("message-arg")
                .short("a")
                .multiple(true)
                .number_of_values(1)
                .help("Arguments for the event message."),
        )
        .arg(
            Arg::with_name("platform")
                .value_name("PLATFORM")
                .long("platform")
                .short("p")
                .help("Override the default 'other' platform specifier."),
        )
        .arg(
            Arg::with_name("tags")
                .value_name("KEY:VALUE")
                .long("tag")
                .short("t")
                .multiple(true)
                .number_of_values(1)
                .help("Add a tag (key:value) to the event."),
        )
        .arg(
            Arg::with_name("extra")
                .value_name("KEY:VALUE")
                .long("extra")
                .short("e")
                .multiple(true)
                .number_of_values(1)
                .help("Add extra information (key:value) to the event."),
        )
        .arg(
            Arg::with_name("user_data")
                .value_name("KEY:VALUE")
                .long("user")
                .short("u")
                .multiple(true)
                .number_of_values(1)
                .help(
                    "Add user information (key:value) to the event. \
                     [eg: id:42, username:foo]",
                ),
        )
        .arg(
            Arg::with_name("fingerprint")
                .value_name("FINGERPRINT")
                .long("fingerprint")
                .short("f")
                .multiple(true)
                .number_of_values(1)
                .help("Change the fingerprint of the event."),
        )
        .arg(
            Arg::with_name("logfile")
                .value_name("PATH")
                .long("logfile")
                .help("Send a logfile as breadcrumbs with the event (last 100 records)"),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::get_current();
    let mut event = Event::default();

    event.sdk_info = Some(Cow::Owned(ClientSdkInfo {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        integrations: Vec::new(),
    }));

    event.level = matches
        .value_of("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(Level::Error);

    if let Some(release) = matches.value_of("release") {
        event.release = Some(release.into());
    } else {
        event.release = detect_release_name().ok().map(|r| r.into());
    }

    event.dist = matches.value_of("dist").map(|x| x.into());
    event.platform = matches.value_of("platform").unwrap_or("other").into();
    event.environment = matches.value_of("environment").map(|x| x.into());

    if let Some(mut lines) = matches.values_of("message") {
        event.logentry = Some(LogEntry {
            message: lines.join("\n"),
            params: if let Some(args) = matches.values_of("message_args") {
                args.map(|x| x.into()).collect()
            } else {
                vec![]
            },
        });
    }

    if let Some(tags) = matches.values_of("tags") {
        for tag in tags {
            let mut split = tag.splitn(2, ':');
            let key = split.next().ok_or_else(|| err_msg("missing tag key"))?;
            let value = split.next().ok_or_else(|| err_msg("missing tag value"))?;
            event.tags.insert(key.into(), value.into());
        }
    }

    if matches.is_present("no-environ") {
        event.extra.remove("environ");
    }
    if let Some(extra) = matches.values_of("extra") {
        for pair in extra {
            let mut split = pair.splitn(2, ':');
            let key = split.next().ok_or_else(|| err_msg("missing extra key"))?;
            let value = split.next().ok_or_else(|| err_msg("missing extra value"))?;
            event.extra.insert(key.into(), Value::String(value.into()));
        }
    }

    if let Some(user_data) = matches.values_of("user_data") {
        let user = User::default();
        for pair in user_data {
            let mut split = pair.splitn(2, ':');
            let key = split.next().ok_or_else(|| err_msg("missing user key"))?;
            let value = split.next().ok_or_else(|| err_msg("missing user value"))?;

            match key {
                "id" => user.id = Some(value.into()),
                "email" => user.email = Some(value.into()),
                "ip_address" => user.ip_address = Some(value.parse()?),
                "username" => user.username = Some(value.into()),
                _ => {
                    user.data.insert(key.into(), value.into());
                }
            };
        }

        event.user = Some(user);
    }

    if let Some(fingerprint) = matches.values_of("fingerprint") {
        event.fingerprint = fingerprint.map(|x| x.into()).collect().into();
    }

    if let Some(logfile) = matches.value_of("logfile") {
        attach_logfile(&mut event, logfile, false)?;
    }

    let client = Client::with_dsn(config.get_dsn()?);
    let event_id = client.capture_event(event, None);
    if client.drain_events(None) {
        println!("Event sent: {}", event_id);
    } else {
        println_stderr!("error: could not send event");
        return Err(QuietExit(1).into());
    }

    Ok(())
}
