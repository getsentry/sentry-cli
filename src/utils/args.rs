use std::result::Result as StdResult;

use uuid::Uuid;
use clap;
use chrono::{DateTime, UTC, TimeZone};

use prelude::*;


fn validate_org(v: String) -> StdResult<(), String> {
    if v.contains("/") || &v == "." || &v == ".." || v.contains(' ') {
        return Err("invalid value for organization. Use the URL \
                    slug and not the name!".into())
    } else {
        Ok(())
    }
}

pub fn validate_project(v: String) -> StdResult<(), String> {
    if v.contains("/") || &v == "." || &v == ".." ||
       v.contains(' ') || v.contains('\n') || v.contains('\t') || v.contains('\r') {
        return Err("invalid value for project. Use the URL \
                    slug and not the name!".into())
    } else {
        Ok(())
    }
}

fn validate_version(v: String) -> StdResult<(), String> {
    if v.trim() != &v {
        Err(format!("Invalid release version. Releases must not contain \
                     leading or trailing spaces."))
    } else if v.len() == 0 || &v == "." || &v == ".." ||
       v.find(&['\n', '\t', '\x0b', '\x0c', '\t', '/'][..]).is_some() {
        Err(format!("Invalid release version. Slashes and certain \
                     whitespace characters are not permitted."))
    } else {
        Ok(())
    }
}

pub fn validate_seconds(v: String) -> StdResult<(), String> {
    if v.parse::<i64>().is_ok() {
        Ok(())
    } else {
        Err(format!("Invalid value (seconds as integer required)"))
    }
}

pub fn validate_timestamp(v: String) -> StdResult<(), String> {
    if let Err(err) = get_timestamp(&v) {
        Err(err.to_string())
    } else {
        Ok(())
    }
}

pub fn validate_uuid(s: String) -> StdResult<(), String> {
    if Uuid::parse_str(&s).is_err() {
        Err("Invalid UUID".to_string())
    } else {
        Ok(())
    }
}

pub fn get_timestamp(value: &str) -> Result<DateTime<UTC>> {
    if let Ok(int) = value.parse::<i64>() {
        Ok(UTC.timestamp(int, 0))
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        Ok(dt.with_timezone(&UTC))
    } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        Ok(dt.with_timezone(&UTC))
    } else {
        Err(Error::from("not in valid format. Unix timestamp or ISO 8601 date expected."))
    }
}

pub trait ArgExt: Sized {
    fn org_arg(self) -> Self;
    fn project_arg(self) -> Self;
    fn projects_arg(self) -> Self;
    fn org_project_args(self) -> Self {
        self.org_arg().project_arg()
    }
    fn version_arg(self, index: u64) -> Self;
}

impl<'a: 'b, 'b> ArgExt for clap::App<'a, 'b> {
    fn org_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("org")
            .value_name("ORG")
            .long("org")
            .short("o")
            .validator(validate_org)
            .help("The organization slug"))
    }

    fn project_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("project")
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .validator(validate_project)
            .help("The project slug"))
    }

    fn projects_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("projects")
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .multiple(true)
            .number_of_values(1)
            .required(false)
            .validator(validate_project)
            .help("The project slug.  This can be supplied multiple times."))
    }

    fn version_arg(self, index: u64) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("version")
            .value_name("VERSION")
            .required(true)
            .index(index)
            .validator(validate_version)
            .help("The version of the release"))
    }
}
