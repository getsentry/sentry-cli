//! Implements a command for sending events to Sentry.
use std::borrow::Cow;
use std::env;

use clap::{App, Arg, ArgMatches};
use failure::{err_msg, Error};
use itertools::Itertools;
use sentry::protocol::{Event, Level, LogEntry, User};
use serde_json::Value;
use username::get_user_name;

use crate::config::Config;
use crate::utils::event::{attach_logfile, get_sdk_info, with_sentry_client};
use crate::utils::releases::detect_release_name;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Send a manual event to Sentry.")
        .long_about(
            "Send a manual event to Sentry.{n}{n}\
             NOTE: This command will validate input parameters and attempt to send an event to \
             Sentry. Due to network errors, rate limits or sampling the event is not guaranteed to \
             actually arrive. Check debug output for transmission errors by passing --log-level=\
             debug or setting `SENTRY_LOG_LEVEL=debug`.",
        )
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
        .arg(
            Arg::with_name("with_categories")
                .long("with-categories")
                .help("Parses off a leading category for breadcrumbs from the logfile"),
        )
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::current();
    let mut event = Event::default();

    event.sdk = Some(get_sdk_info());

    event.level = matches
        .value_of("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(Level::Error);

    if let Some(release) = matches.value_of("release") {
        event.release = Some(release.to_string().into());
    } else {
        event.release = detect_release_name().ok().map(Cow::from);
    }

    event.dist = matches.value_of("dist").map(|x| x.to_string().into());
    event.platform = matches
        .value_of("platform")
        .unwrap_or("other")
        .to_string()
        .into();
    event.environment = matches
        .value_of("environment")
        .map(|x| x.to_string().into());

    if let Some(mut lines) = matches.values_of("message") {
        event.logentry = Some(LogEntry {
            message: lines.join("\n"),
            params: matches
                .values_of("message_args")
                .map(|args| args.map(|x| x.into()).collect())
                .unwrap_or_default(),
        });
    }

    for tag in matches.values_of("tags").unwrap_or_default() {
        let mut split = tag.splitn(2, ':');
        let key = split.next().ok_or_else(|| err_msg("missing tag key"))?;
        let value = split.next().ok_or_else(|| err_msg("missing tag value"))?;
        event.tags.insert(key.into(), value.into());
    }

    if !matches.is_present("no_environ") {
        event.extra.insert(
            "environ".into(),
            Value::Object(env::vars().map(|(k, v)| (k, Value::String(v))).collect()),
        );
    }

    for pair in matches.values_of("extra").unwrap_or_default() {
        let mut split = pair.splitn(2, ':');
        let key = split.next().ok_or_else(|| err_msg("missing extra key"))?;
        let value = split.next().ok_or_else(|| err_msg("missing extra value"))?;
        event.extra.insert(key.into(), Value::String(value.into()));
    }

    if let Some(user_data) = matches.values_of("user_data") {
        let mut user = User::default();
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

    if let Some(fingerprint) = matches.values_of("fingerprint") {
        event.fingerprint = fingerprint
            .map(|x| x.to_string().into())
            .collect::<Vec<_>>()
            .into();
    }

    if let Some(logfile) = matches.value_of("logfile") {
        attach_logfile(&mut event, logfile, matches.is_present("with_categories"))?;
    }

    let id = with_sentry_client(config.get_dsn()?, |c| c.capture_event(event, None));
    println!("{}", id);

    Ok(())
}
