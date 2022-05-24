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
    println!("{}", style(format!("ℹ {}", msg)).blue());
}

fn success<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("✔ {}", msg)).green());
}

fn warning<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("⚠ {}", msg)).yellow());
}

fn error<S>(msg: S)
where
    S: std::fmt::Display,
{
    println!("{}", style(format!("✖ {}", msg)).red());
}

fn fetch_event(org: &str, project: &str, event_id: &str) -> Result<ProcessedEvent> {
    match Api::current().get_event(org, Some(project), event_id)? {
        Some(event) => {
            success(format!("Fetched data for event: {}", event_id));
            Ok(event)
        }
        None => {
            error(format!("Could not retrieve event {}", event_id));
            tip("Make sure that event ID you used is valid.");
            Err(QuietExit(1).into())
        }
    }
}

fn extract_top_frame(stacktrace: &Stacktrace) -> Result<&Frame> {
    let in_app_frames: Vec<&Frame> = stacktrace
        .frames
        .iter()
        .filter(|frame| frame.in_app.unwrap_or(false))
        .collect();

    if in_app_frames.is_empty() {
        bail!("Event exception stacktrace has no in_app frames");
    }

    let top_frame = in_app_frames.last().unwrap();
    let abs_path = top_frame
        .abs_path
        .as_ref()
        .ok_or_else(|| format_err!("Top frame is missing an abs_path"))?;

    if let Ok(abs_path) = Url::parse(abs_path) {
        if Path::new(abs_path.path()).extension().is_none() {
            bail!("Top frame of event exception originates from the <script> tag, its not possible to resolve source maps");
        }
    } else {
        bail!("Event exception stacktrace top frame has incorrect abs_path (valid url is required). Found {}", abs_path);
    }

    Ok(top_frame)
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
//
// We only need the `pathname` portion of the url, so if it's absolute, just extract it.
// If it's relative however, parse any random url (example.com) and join it with our relative url,
// as Rust cannot handle parsing of relative urls.
//
// http://localhost:5000/dist/bundle.min.js => ~/dist/bundle.min.js
// /dist/bundle.js.map => ~/dist/bundle.js.map
// okboomer => error (invalid relative path, no extension)
//
// It should be more generic than using the defaults, but should be sufficient for our current usecase.
fn find_matching_artifact(artifacts: &[Artifact], abs_path: &str) -> Result<Artifact> {
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

    let full_match = artifacts.iter().find(|a| a.name == filename);
    let partial_match = artifacts
        .iter()
        .find(|a| a.name.ends_with(filename.split('/').last().unwrap()));

    if full_match.is_none() {
        error(format!(
            "Uploaded artifacts do not include entry: {}",
            filename
        ));

        if let Some(pm) = partial_match {
            tip(format!(
                "Found entry with partially matching filename: {}. \
                Make sure that that --url-prefix is set correctly.",
                pm.name,
            ));
        }

        return Err(QuietExit(1).into());
    }

    success(format!("Artifact {} found.", filename));
    Ok(full_match.cloned().unwrap())
}

fn verify_dists_matches(artifact: &Artifact, dist: Option<&str>) -> Result<()> {
    if artifact.dist.as_deref() != dist {
        error(format!(
            "Release artifact distrubition mismatch. Event: {}, Artifact: {}",
            dist.unwrap_or("[none]"),
            artifact.dist.as_ref().unwrap_or(&String::from("[none]"))
        ));
        tip("Configure 'dist' option in the SDK to match the one used during artifacts upload.\n  \
            https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/#verify-artifact-distribution-value-matches-value-configured-in-your-sdk");

        return Err(QuietExit(1).into());
    }

    success(format!(
        "Release artifact distrubition matched. Event: {}, Artifact: {}",
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
                .map(|l| format!("  {}", l))
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
                .map(|l| format!("  {}", l))
                .collect::<Vec<String>>()
                .join("\n")
        )
        .yellow()
    );
}

fn extract_release(event: &ProcessedEvent) -> Result<String> {
    if let Some(release) = event.release.as_ref() {
        success(format!("Event has release name: {}", release));
        Ok(release.to_string())
    } else {
        error("Event is missing a release name");
        tip("Configure 'release' option in the SDK.\n  \
            https://docs.sentry.io/platforms/javascript/configuration/options/#release\n  \
            https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/#verify-a-release-is-configured-in-your-sdk");
        Err(QuietExit(1).into())
    }
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let (org, project) = config.get_org_and_project(matches)?;
    let event_id = matches.value_of("event").unwrap();

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

    let mut frame = extract_top_frame(stacktrace).map_err(|err| {
        error(err);
        QuietExit(1)
    })?;

    if exception.raw_stacktrace.is_some() {
        if matches.is_present("force") {
            warning(
                "Exception is already source mapped, however 'force' flag was used. Moving along.",
            );
            let raw_stacktrace = exception.raw_stacktrace.as_ref().unwrap();
            frame = extract_top_frame(raw_stacktrace).map_err(|err| {
                error(err);
                QuietExit(1)
            })?;
        } else {
            warning("Exception is already source mapped and resolves to:\n");
            print_mapped_frame(frame);
            return Err(QuietExit(0).into());
        }
    }

    let artifacts = fetch_release_artifacts(&org, &project, &release)?;
    let matched_artifact = find_matching_artifact(&artifacts, frame.abs_path.as_ref().unwrap())?;

    verify_dists_matches(&matched_artifact, event.dist.as_deref())?;

    let sourcemap_location =
        discover_sourcemaps_location(&org, &project, &release, &matched_artifact).map_err(
            |err| {
                error(err);
                QuietExit(1)
            },
        )?;
    success(format!("Found source map location: {}", sourcemap_location));

    let sourcemap_artifact = find_matching_artifact(&artifacts, &sourcemap_location)?;
    verify_dists_matches(&sourcemap_artifact, event.dist.as_deref())?;

    let sourcemap_file =
        fetch_release_artifact_file(&org, &project, &release, &sourcemap_artifact)?;

    print_sourcemap(
        &sourcemap_file,
        frame.lineno.unwrap() as u32 - 1,
        frame.colno.unwrap() as u32 - 1,
    )
    .map_err(|err| {
        error(err);
        QuietExit(1)
    })?;

    success("Source Maps should be working fine. Have you tried turning it off and on again?");
    Ok(())
}
