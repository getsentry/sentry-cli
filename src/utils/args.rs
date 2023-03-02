use anyhow::{bail, Result};
use chrono::{DateTime, TimeZone, Utc};
use clap::{Arg, ArgAction, Command};

fn validate_org(v: &str) -> Result<String, String> {
    if v.contains('/') || v == "." || v == ".." || v.contains(' ') {
        Err("Invalid value for organization. Use the URL slug and not the name!".to_string())
    } else {
        Ok(v.to_owned())
    }
}

pub fn validate_project(v: &str) -> Result<String, String> {
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
        Ok(v.to_owned())
    }
}

fn validate_release(v: &str) -> Result<String, String> {
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
        Ok(v.to_owned())
    }
}

pub fn validate_distribution(v: &str) -> Result<String, String> {
    if v.trim() != v {
        Err(
            "Invalid distribution name. Distribution must not contain leading or trailing spaces."
                .to_string(),
        )
    } else if bytecount::num_chars(v.as_bytes()) > 64 {
        Err(
            "Invalid distribution name. Distribution name must not be longer than 64 characters."
                .to_string(),
        )
    } else {
        Ok(v.to_owned())
    }
}

pub fn get_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(int) = value.parse::<i64>() {
        Ok(Utc.timestamp_opt(int, 0).single().unwrap())
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
    fn release_arg(self) -> Self;
    fn version_arg(self) -> Self;
}

impl<'a: 'b, 'b> ArgExt for Command<'a> {
    fn org_arg(self) -> Command<'a> {
        self.arg(
            Arg::new("org")
                .value_name("ORG")
                .long("org")
                .short('o')
                .value_parser(validate_org)
                .global(true)
                .help("The organization slug"),
        )
    }

    fn project_arg(self, multiple: bool) -> Command<'a> {
        self.arg(
            Arg::new("project")
                .value_name("PROJECT")
                .long("project")
                .short('p')
                .value_parser(validate_project)
                .global(true)
                .action(if multiple {
                    ArgAction::Append
                } else {
                    ArgAction::Set
                })
                .help("The project slug."),
        )
    }

    fn release_arg(self) -> Command<'a> {
        self.arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .short('r')
                .global(true)
                .allow_hyphen_values(true)
                .value_parser(validate_release)
                .help("The release slug."),
        )
    }

    fn version_arg(self) -> Command<'a> {
        self.arg(
            Arg::new("version")
                .value_name("VERSION")
                .required(true)
                .value_parser(validate_release)
                .help("The version of the release"),
        )
    }
}
