use std::io::Read;
use std::path::Path;

use anyhow::{bail, format_err, Result};
use clap::{Arg, ArgMatches, Command};
use console::style;
use sentry::protocol::{Frame, Stacktrace};
use url::Url;

use crate::api::{Api, Artifact, ProcessedEvent};
use crate::config::Config;
use crate::utils::fs::TempFile;
use crate::utils::system::QuietExit;

use super::resolve::print_source;

pub fn make_command(command: Command) -> Command {
    command
        .about("Explain why sourcemaps are not working for a given event.")
        .alias("why")
        .arg(
            Arg::new("event")
                .value_name("EVENT_ID")
                .required(true)
                .help("ID of an event to be explained."),
        )
        .arg(
            Arg::new("frame")
                .long("frame")
                .default_value("0")
                .value_parser(clap::value_parser!(usize))
                .help("Position of the frame that should be used for source map resolution."),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help("Force full validation flow, even when event is already source mapped."),
        )
}

fn tip<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("ℹ {msg}")).blue());
}

fn success<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("✔ {msg}")).green());
}

fn warning<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("⚠ {msg}")).yellow());
}

fn error<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("✖ {msg}")).red());
}

fn fetch_event(org: &str, project: &str, event_id: &str) -> Result<ProcessedEvent> {
    match Api::current().get_event(org, Some(project), event_id)? {
        Some(event) => {
            success(format!("Fetched data for event: {event_id}"));
            Ok(event)
        }
        None => {
            error(format!("Could not retrieve event {event_id}"));
            tip("Make sure that event ID you used is valid.");
            Err(QuietExit(1).into())
        }
    }
}

fn extract_in_app_frames(stacktrace: &Stacktrace) -> Vec<&Frame> {
    stacktrace
        .frames
        .iter()
        .filter(|frame| frame.in_app.unwrap_or(false))
        .collect()
}

fn extract_nth_frame(stacktrace: &Stacktrace, position: usize) -> Result<&Frame> {
    let mut in_app_frames = extract_in_app_frames(stacktrace);

    if in_app_frames.is_empty() {
        bail!("Event exception stacktrace has no in_app frames");
    }

    // Frames are in bottom-up order.
    in_app_frames.reverse();

    let frame = in_app_frames
        .get(position)
        .ok_or_else(|| format_err!("Selected frame ({}) is missing.", position))?;

    let abs_path = frame
        .abs_path
        .as_ref()
        .ok_or_else(|| format_err!("Selected frame ({}) is missing an abs_path", position))?;

    if let Ok(abs_path) = Url::parse(abs_path) {
        if Path::new(abs_path.path()).extension().is_none() {
            bail!("Selected frame ({}) of event exception originates from the <script> tag, its not possible to resolve source maps", position);
        }
    } else {
        bail!("Event exception stacktrace selected frame ({}) has incorrect abs_path (valid url is required). Found {}", position, abs_path);
    }

    Ok(frame)
}

fn fetch_release_artifacts(org: &str, project: &str, release: &str) -> Result<Vec<Artifact>> {
    Api::current().list_release_files(org, Some(project), release).map(|artifacts| {
        if artifacts.is_empty() {
            error("Release has no artifacts uploaded");
            tip("https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/#verify-artifacts-are-uploaded");
            return Err(QuietExit(1).into());
        }
        Ok(artifacts)
    })?
}

// Try to find an artifact which matches the path part of the url extracted from the stacktrace frame,
// prefixed with the default `~/`, which is a "glob-like" pattern for matchin any hostname.
fn find_matching_artifact(artifacts: &[Artifact], path: &str) -> Result<Artifact> {
    let full_match = artifacts.iter().find(|a| a.name == path);
    let partial_match = artifacts
        .iter()
        .find(|a| a.name.ends_with(path.split('/').last().unwrap()));

    if full_match.is_none() {
        error(format!("Uploaded artifacts do not include entry: {path}"));

        if let Some(pm) = partial_match {
            tip(format!(
                "Found entry with partially matching filename: {}. \
                Make sure that that --url-prefix is set correctly.",
                pm.name,
            ));
        }

        return Err(QuietExit(1).into());
    }

    success(format!("Artifact {path} found."));
    Ok(full_match.cloned().unwrap())
}

fn verify_dists_matches(artifact: &Artifact, dist: Option<&str>) -> Result<()> {
    if artifact.dist.as_deref() != dist {
        error(format!(
            "Release artifact distribution mismatch. Event: {}, Artifact: {}",
            dist.unwrap_or("[none]"),
            artifact.dist.as_ref().unwrap_or(&String::from("[none]"))
        ));
        tip("Configure 'dist' option in the SDK to match the one used during artifacts upload.\n  \
            https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/#verify-artifact-distribution-value-matches-value-configured-in-your-sdk");

        return Err(QuietExit(1).into());
    }

    success(format!(
        "Release artifact distribution matched. Event: {}, Artifact: {}",
        dist.unwrap_or("[none]"),
        artifact.dist.as_ref().unwrap_or(&String::from("[none]"))
    ));
    Ok(())
}

fn fetch_release_artifact_file(
    org: &str,
    project: &str,
    release: &str,
    artifact: &Artifact,
) -> Result<TempFile> {
    let api = Api::current();
    let file = TempFile::create()?;

    api.get_release_file(
        org,
        Some(project),
        release,
        &artifact.id,
        &mut file.open().unwrap(),
    )
    .map(|_| {
        success(format!(
            "Successfully fetched {} file from the server.",
            artifact.name
        ));
        Ok(file)
    })
    .map_err(|err| {
        format_err!(
            "Could not retrieve file {} from release {}: {:?}",
            artifact.name,
            release,
            err
        )
    })?
}

fn fetch_release_artifact_file_metadata(
    org: &str,
    project: &str,
    release: &str,
    artifact: &Artifact,
) -> Result<Artifact> {
    let api = Api::current();
    let file_metadata = api.get_release_file_metadata(org, Some(project), release, &artifact.id)?;
    file_metadata
        .ok_or_else(|| format_err!("Could not retrieve file metadata: {}", &artifact.id))
        .map(|f| {
            success(format!(
                "Successfully fetched {} file metadata from the server.",
                artifact.name
            ));
            f
        })
}

// https://github.com/getsentry/sentry/blob/623c2f5f3313e6dc55e08e2ae2b11d8f90cdbece/src/sentry/lang/javascript/processor.py#L145-L207
fn discover_sourcemaps_location(
    org: &str,
    project: &str,
    release: &str,
    artifact: &Artifact,
) -> Result<String> {
    let file_metadata = fetch_release_artifact_file_metadata(org, project, release, artifact)?;

    if let Some(header) = file_metadata.headers.get("Sourcemap") {
        return Ok(header.to_owned());
    }

    if let Some(header) = file_metadata.headers.get("X-SourceMap") {
        return Ok(header.to_owned());
    }

    let file = fetch_release_artifact_file(org, project, release, artifact)?;

    let mut f = file.open()?;
    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;

    for line in buffer.lines().rev() {
        if line.starts_with("//# sourceMappingURL=") || line.starts_with("//@ sourceMappingURL=") {
            let possible_sourcemap = line[21..].trim();
            if possible_sourcemap.starts_with("data:application/json") {
                bail!("Found inlined source maps, this tool doesnt support further verification for this scenario yet");
            }
            return Ok(possible_sourcemap.to_owned());
        }
    }

    Err(format_err!("Failed to discover source map url"))
}

fn print_sourcemap(file: &TempFile, line: u32, column: u32) -> Result<()> {
    let mut f = file.open()?;
    let mut buf = vec![];
    f.read_to_end(&mut buf)?;
    let sm = sourcemap::decode_slice(&buf)?;

    if let Some(token) = sm.lookup_token(line, column) {
        if let Some(view) = token.get_source_view() {
            success("Sourcemap position resolves to:");
            print_source(&token, view);
        } else if token.get_source_view().is_none() {
            bail!("cannot find source");
        } else {
            bail!("cannot find source for line {} column {}", line, column);
        }
    } else {
        bail!("invalid sourcemap location");
    }

    Ok(())
}

fn print_mapped_frame(frame: &Frame) {
    println!(
        "{}",
        style(
            frame
                .pre_context
                .iter()
                .map(|l| format!("  {l}"))
                .collect::<Vec<String>>()
                .join("\n")
        )
        .yellow()
    );
    println!(
        "{}",
        style(format!("> {}", frame.context_line.as_ref().unwrap())).yellow()
    );
    println!(
        "{}",
        style(
            frame
                .post_context
                .iter()
                .map(|l| format!("  {l}"))
                .collect::<Vec<String>>()
                .join("\n")
        )
        .yellow()
    );
}

fn extract_release(event: &ProcessedEvent) -> Result<String> {
    if let Some(release) = event.release.as_ref() {
        success(format!("Event has release name: {release}"));
        Ok(release.to_string())
    } else {
        error("Event is missing a release name");
        tip("Configure 'release' option in the SDK.\n  \
            https://docs.sentry.io/platforms/javascript/configuration/options/#release\n  \
            https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/#verify-a-release-is-configured-in-your-sdk");
        Err(QuietExit(1).into())
    }
}

fn resolve_sourcemap_url(abs_path: &str, sourcemap_location: &str) -> Result<String> {
    let base = Url::parse(abs_path)?;
    base.join(sourcemap_location)
        .map(|url| url.to_string())
        .map_err(|e| e.into())
}

// Unify url to be prefixed with the default `~/`, which is a "glob-like" pattern for matchin any hostname.
//
// We only need the `pathname` portion of the url, so if it's absolute, just extract it.
// If it's relative however, parse any random url (example.com) and join it with our relative url,
// as Rust cannot handle parsing of relative urls.
//
// It should be more generic than using the defaults, but should be sufficient for our current usecase.
fn unify_artifact_url(abs_path: &str) -> Result<String> {
    let abs_path = match Url::parse(abs_path) {
        Ok(path) => Ok(path),
        Err(_) => {
            let base = Url::parse("http://example.com").unwrap();
            base.join(abs_path)
                .map_err(|_| format_err!("Cannot parse source map url {}", abs_path))
        }
    }?;
    let mut filename = String::from("~");
    filename.push_str(abs_path.path());
    Ok(filename)
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let event_id = matches.get_one::<String>("event").unwrap();

    let event = fetch_event(&org, &project, event_id)?;
    let release = extract_release(&event)?;

    if event.exception.values.is_empty() {
        warning("Event has no exception captured, there is no use for source maps");
        return Err(QuietExit(0).into());
    }
    success("Event has a valid exception present");

    let exception = &event.exception.values[0];
    let stacktrace = exception.stacktrace.as_ref().ok_or_else(|| {
        error("Event exception has no stacktrace available");
        QuietExit(1)
    })?;
    success("Event has a valid stacktrace present");

    let mut frame = extract_nth_frame(stacktrace, *matches.get_one::<usize>("frame").unwrap())
        .map_err(|err| {
            error(err);
            QuietExit(1)
        })?;

    if exception.raw_stacktrace.is_some() {
        if matches.contains_id("force") {
            warning(
                "Exception is already source mapped, however 'force' flag was used. Moving along.",
            );
            let raw_stacktrace = exception.raw_stacktrace.as_ref().unwrap();
            frame = extract_nth_frame(raw_stacktrace, *matches.get_one::<usize>("frame").unwrap())
                .map_err(|err| {
                    error(err);
                    QuietExit(1)
                })?;
        } else {
            warning("Exception is already source mapped and first resolved frame points to:\n");
            if let Some(frame) = extract_in_app_frames(stacktrace)
                .iter()
                .rev()
                .find(|f| f.context_line.is_some())
            {
                print_mapped_frame(frame);
            } else {
                println!("{}", style("> [missing context line]").yellow());
            }
            return Err(QuietExit(0).into());
        }
    }

    let abs_path = frame.abs_path.as_ref().expect("Incorrect abs_path value");
    let artifacts = fetch_release_artifacts(&org, &project, &release)?;
    let matched_artifact = find_matching_artifact(&artifacts, &unify_artifact_url(abs_path)?)?;

    verify_dists_matches(&matched_artifact, event.dist.as_deref())?;

    let sourcemap_location =
        discover_sourcemaps_location(&org, &project, &release, &matched_artifact).map_err(
            |err| {
                error(err);
                QuietExit(1)
            },
        )?;
    success(format!(
        "Found source map location: {}",
        &sourcemap_location
    ));

    let sourcemap_url = unify_artifact_url(&resolve_sourcemap_url(abs_path, &sourcemap_location)?)?;
    success(format!("Resolved source map url: {}", &sourcemap_url));

    let sourcemap_artifact = find_matching_artifact(&artifacts, &sourcemap_url)?;
    verify_dists_matches(&sourcemap_artifact, event.dist.as_deref())?;

    let sourcemap_file =
        fetch_release_artifact_file(&org, &project, &release, &sourcemap_artifact)?;

    print_sourcemap(
        &sourcemap_file,
        frame.lineno.expect("Event frame is missing line number") as u32 - 1,
        frame.colno.expect("Event frame is missing column number") as u32 - 1,
    )
    .map_err(|err| {
        error(err);
        QuietExit(1)
    })?;

    success("Source Maps should be working fine. Have you tried turning it off and on again?");
    Ok(())
}

#[test]
fn test_resolve_sourcemap_url() {
    // Tests coming from `tests/sentry/utils/test_urls.py` in `getsentry/sentry`
    let cases = vec![
        ("http://example.com/foo", "bar", "http://example.com/bar"),
        ("http://example.com/foo", "/bar", "http://example.com/bar"),
        ("https://example.com/foo", "/bar", "https://example.com/bar"),
        (
            "http://example.com/foo/baz",
            "bar",
            "http://example.com/foo/bar",
        ),
        (
            "http://example.com/foo/baz",
            "/bar",
            "http://example.com/bar",
        ),
        ("aps://example.com/foo", "/bar", "aps://example.com/bar"),
        (
            "apsunknown://example.com/foo",
            "/bar",
            "apsunknown://example.com/bar",
        ),
        (
            "apsunknown://example.com/foo",
            "//aha/uhu",
            "apsunknown://aha/uhu",
        ),
    ];

    for (base, to_join, expected) in cases {
        assert_eq!(resolve_sourcemap_url(base, to_join).unwrap(), expected);
    }
}

#[test]
fn test_unify_artifact_url() {
    let cases = vec![
        (
            "http://localhost:5000/dist/bundle.min.js",
            "~/dist/bundle.min.js",
        ),
        ("/dist/bundle.js.map", "~/dist/bundle.js.map"),
    ];

    for (path, expected) in cases {
        assert_eq!(unify_artifact_url(path).unwrap(), expected);
    }
}
