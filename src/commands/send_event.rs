//! Implements a command for sending events to Sentry.
use std::fs;
use std::io::{BufRead, BufReader};
use std::env;
use std::collections::HashMap;

use clap::{App, Arg, ArgMatches};
use itertools::Itertools;
use username::get_user_name;
use hostname::get_hostname;
#[cfg(not(windows))]
use uname::uname;
use serde_json::Value;
use anylog::LogEntry;

use prelude::*;
use config::Config;
use event::{Event, Message, Breadcrumb};
use api::Api;
use constants::{ARCH, PLATFORM};
use utils::{get_model, get_family, detect_release_name};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Send a manual event to Sentry.")
        .arg(Arg::with_name("level")
            .value_name("LEVEL")
            .long("level")
            .short("l")
            .help("Optional event severity/log level. [defaults to 'error']"))
        .arg(Arg::with_name("release")
            .value_name("RELEASE")
            .long("release")
            .short("r")
            .help("Optional identifier of the release."))
        .arg(Arg::with_name("dist")
            .value_name("DISTRIBUTION")
            .long("dist")
            .short("d")
            .help("Set the distribution."))
        .arg(Arg::with_name("environment")
            .value_name("ENVIRONMENT")
            .long("env")
            .short("E")
            .help("Send with a specific environment."))
        .arg(Arg::with_name("no_environ")
             .long("no-environ")
             .help("Do not send environment variables along"))
        .arg(Arg::with_name("message")
            .value_name("MESSAGE")
            .long("message")
            .short("m")
            .multiple(true)
            .number_of_values(1)
            .help("The event message."))
        .arg(Arg::with_name("message_args")
            .value_name("MESSAGE_ARG")
            .long("message-arg")
            .short("a")
            .multiple(true)
            .number_of_values(1)
            .help("Arguments for the event message."))
        .arg(Arg::with_name("platform")
            .value_name("PLATFORM")
            .long("platform")
            .short("p")
            .help("Override the default 'other' platform specifier."))
        .arg(Arg::with_name("tags")
            .value_name("KEY:VALUE")
            .long("tag")
            .short("t")
            .multiple(true)
            .number_of_values(1)
            .help("Add a tag (key:value) to the event."))
        .arg(Arg::with_name("extra")
            .value_name("KEY:VALUE")
            .long("extra")
            .short("e")
            .multiple(true)
            .number_of_values(1)
            .help("Add extra information (key:value) to the event."))
        .arg(Arg::with_name("user_data")
             .value_name("KEY:VALUE")
             .long("user")
             .short("u")
            .multiple(true)
            .number_of_values(1)
            .help("Add user information (key:value) to the event. \
                   [eg: id:42, username:foo]"))
        .arg(Arg::with_name("fingerprint")
            .value_name("FINGERPRINT")
            .long("fingerprint")
            .short("f")
            .multiple(true)
            .number_of_values(1)
            .help("Change the fingerprint of the event."))
        .arg(Arg::with_name("logfile")
            .value_name("PATH")
            .long("logfile")
            .help("Send a logfile as breadcrumbs with the event (last 100 records)"))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let mut event = Event::new();
    event.level = matches.value_of("level").unwrap_or("error").into();
    event.release = matches.value_of("release").map(|x| x.into());
    if event.release.is_none() {
        event.release = detect_release_name().ok();
    }
    event.dist = matches.value_of("dist").map(|x| x.into());
    event.platform = matches.value_of("platform").unwrap_or("other").into();
    event.environment = matches.value_of("environment").map(|x| x.into());

    if let Some(mut lines) = matches.values_of("message") {
        event.message = Some(Message {
            message: lines.join("\n"),
            params: if let Some(args) = matches.values_of("message_args") {
                args.map(|x| x.to_string()).collect()
            } else {
                vec![]
            },
        });
    }

    if let Some(tags) = matches.values_of("tags") {
        for tag in tags {
            let mut split = tag.splitn(2, ':');
            let key = split.next().ok_or("missing tag key")?;
            let value = split.next().ok_or("missing tag value")?;
            event.tags.insert(key.into(), value.into());
        }
    }

    if !matches.is_present("no-environ") {
        event.extra.insert("environ".into(), Value::Object(env::vars().map(|(k, v)| {
            (k, Value::String(v))
        }).collect()));
    }

    if let Some(extra) = matches.values_of("extra") {
        for pair in extra {
            let mut split = pair.splitn(2, ':');
            let key = split.next().ok_or("missing extra key")?;
            let value = split.next().ok_or("missing extra value")?;
            event.extra.insert(key.into(), Value::String(value.into()));
        }
    }

    if let Some(user_data) = matches.values_of("user_data") {
        for pair in user_data {
            let mut split = pair.splitn(2, ':');
            let key = split.next().ok_or("missing user key")?;
            let value = split.next().ok_or("missing user value")?;
            event.user.insert(key.into(), value.into());
        }
    } else {
        event.user.insert("username".into(), get_user_name().unwrap_or("unknown".into()));
    }

    let mut device = HashMap::new();
    if let Some(hostname) = get_hostname() {
        device.insert("name".into(), hostname);
    }
    if let Some(model) = get_model() {
        device.insert("model".into(), model);
    }
    if let Some(family) = get_family() {
        device.insert("family".into(), family);
    }
    device.insert("arch".into(), ARCH.into());
    event.contexts.insert("device".into(), device);

    let mut os = HashMap::new();
    #[cfg(not(windows))] {
        if let Ok(info) = uname() {
            os.insert("name".into(), info.sysname);
            os.insert("kernel_version".into(), info.version);
            os.insert("version".into(), info.release);
        }
    }
    if !os.contains_key("name") {
        os.insert("name".into(), PLATFORM.into());
    }
    event.contexts.insert("os".into(), os);

    if let Some(fingerprint) = matches.values_of("fingerprint") {
        event.fingerprint = Some(fingerprint.map(|x| x.to_string()).collect());
    }

    if let Some(logfile) = matches.value_of("logfile") {
        let f = fs::File::open(logfile)
            .chain_err(|| "Could not open logfile")?;
        let reader = BufReader::new(f);
        for line in reader.lines() {
            let line = line?;
            let rec = LogEntry::parse(line.as_bytes());
            event.breadcrumbs.push(Breadcrumb {
                timestamp: rec.utc_timestamp().map(|x| x.timestamp() as f64),
                message: rec.message().to_string(),
                ty: "default".to_string(),
                category: "log".to_string(),
            })
        }
    }

    if event.breadcrumbs.len() > 100 {
        let skip = event.breadcrumbs.len() - 100;
        event.breadcrumbs = event.breadcrumbs.into_iter().skip(skip).collect();
    }

    let dsn = config.get_dsn()?;

    // handle errors here locally so that we do not get the extra "use sentry-cli
    // login" to sign in which would be in appropriate here.
    match Api::new(config).send_event(&dsn, &event) {
        Ok(event_id) => {
            println!("Event sent: {}", event_id);
        }
        Err(err) => {
            println!("error: could not send event: {}", err);
        }
    };

    Ok(())
}
