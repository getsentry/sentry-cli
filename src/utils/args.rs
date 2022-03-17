use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use clap::Command;
use failure::{bail, Error};
use symbolic::common::DebugId;
use uuid::Uuid;

fn validate_org(v: &str) -> Result<(), String> {
    if v.contains('/') || v == "." || v == ".." || v.contains(' ') {
        Err("Invalid value for organization. Use the URL slug and not the name!".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_project(v: &str) -> Result<(), String> {
    if v.contains('/')
        || v == "."
        || v == ".."
        || v.contains(' ')
        || v.contains('\n')
        || v.contains('\t')
        || v.contains('\r')
    {
        Err("Invalid value for project. Use the URL slug and not the name!".to_string())
    } else {
        Ok(())
    }
}

fn validate_version(v: &str) -> Result<(), String> {
    if v.trim() != v {
        Err(
            "Invalid release version. Releases must not contain leading or trailing spaces."
                .to_string(),
        )
    } else if v.is_empty()
        || v == "."
        || v == ".."
        || v.find(&['\n', '\t', '\x0b', '\x0c', '\t', '/'][..])
            .is_some()
    {
        Err(
            "Invalid release version. Slashes and certain whitespace characters are not permitted."
                .to_string(),
        )
    } else {
        Ok(())
    }
}

pub fn validate_int(v: &str) -> Result<(), String> {
    if v.parse::<i64>().is_ok() {
        Ok(())
    } else {
        Err("Invalid number, integer required.".to_string())
    }
}

pub fn validate_timestamp(v: &str) -> Result<(), String> {
    if let Err(err) = get_timestamp(v) {
        Err(err.to_string())
    } else {
        Ok(())
    }
}

pub fn validate_uuid(s: &str) -> Result<(), String> {
    if Uuid::parse_str(s).is_err() {
        Err("Invalid UUID.".to_string())
    } else {
        Ok(())
    }
}

pub fn validate_id(s: &str) -> Result<(), String> {
    if DebugId::from_str(s).is_err() {
        Err("Invalid ID.".to_string())
    } else {
        Ok(())
    }
}

pub fn get_timestamp(value: &str) -> Result<DateTime<Utc>, Error> {
    if let Ok(int) = value.parse::<i64>() {
        Ok(Utc.timestamp(int, 0))
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        Ok(dt.with_timezone(&Utc))
    } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        Ok(dt.with_timezone(&Utc))
    } else {
        bail!("Not in valid format. Unix timestamp or ISO 8601 date expected.");
    }
}

pub trait ArgExt: Sized {
    fn org_arg(self) -> Self;
    fn project_arg(self, multiple: bool) -> Self;
    fn version_arg(self, index: usize) -> Self;
}

impl<'a: 'b, 'b> ArgExt for Command<'a> {
    fn org_arg(self) -> Command<'a> {
        self.arg(
            clap::Arg::new("org")
                .value_name("ORG")
                .long("org")
                .short('o')
                .validator(validate_org)
                .global(true)
                .help("The organization slug"),
        )
    }

    fn project_arg(self, multiple: bool) -> Command<'a> {
        self.arg(
            clap::Arg::new("project")
                .value_name("PROJECT")
                .long("project")
                .short('p')
                .validator(validate_project)
                .global(true)
                .multiple_occurrences(multiple)
                .help("The project slug."),
        )
    }

    fn version_arg(self, index: usize) -> Command<'a> {
        self.arg(
            clap::Arg::new("version")
                .value_name("VERSION")
                .required(true)
                .index(index)
                .allow_hyphen_values(true)
                .validator(validate_version)
                .help("The version of the release"),
        )
    }
}
