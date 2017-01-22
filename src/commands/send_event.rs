//! Implements a command for sending events to Sentry.
use clap::{App, Arg, ArgMatches};
use itertools::Itertools;

use prelude::*;
use config::Config;
use event::Event;
use api::Api;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("sends an event to Sentry")
        .arg(Arg::with_name("level")
            .value_name("LEVEL")
            .long("level")
            .short("l")
            .help("Set the log level. Defaults to error"))
        .arg(Arg::with_name("release")
            .value_name("RELEASE")
            .long("release")
            .short("r")
            .help("Set the release."))
        .arg(Arg::with_name("message")
            .value_name("MESSAGE")
            .long("message")
            .short("m")
            .multiple(true)
            .help("Set the message to log"))
        .arg(Arg::with_name("platform")
            .value_name("PLATFORM")
            .long("platform")
            .short("p")
            .help("Overrides the default 'other' platform"))
        .arg(Arg::with_name("tags")
            .value_name("TAG")
            .long("tag")
            .short("t")
            .multiple(true)
            .help("Adds a tag (key:value) to the event."))
        .arg(Arg::with_name("extra")
            .value_name("EXTRA")
            .long("extra")
            .short("e")
            .multiple(true)
            .help("Adds extra information (key:value) to the event."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let mut event = Event::new();
    event.level = matches.value_of("level").unwrap_or("error").into();
    event.release = matches.value_of("release").map(|x| x.into());
    event.platform = matches.value_of("platform").unwrap_or("other").into();
    event.message = matches.values_of("message").map(|mut x| x.join("\n"));

    if let Some(tags) = matches.values_of("tags") {
        for tag in tags {
            let mut split = tag.splitn(2, ':');
            let key = split.next().ok_or("missing tag key")?;
            let value = split.next().ok_or("missing tag value")?;
            event.tags.insert(key.into(), value.into());
        }
    }

    if let Some(extra) = matches.values_of("tags") {
        for pair in extra {
            let mut split = pair.splitn(2, ':');
            let key = split.next().ok_or("missing extra key")?;
            let value = split.next().ok_or("missing extra value")?;
            event.extra.insert(key.into(), value.into());
        }
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
