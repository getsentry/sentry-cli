use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::api::{Api, TraceSpan};
use crate::config::Config;
use crate::utils::args::ArgExt as _;

pub fn make_command(command: Command) -> Command {
    command
        .about("Manage traces in Sentry.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .org_arg()
        .subcommand(
            Command::new("info")
                .about("Get detailed information about a trace.")
                .arg(
                    Arg::new("trace_id")
                        .required(true)
                        .value_name("TRACE_ID")
                        .help("The trace ID."),
                ),
        )
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    if let Some(sub_matches) = matches.subcommand_matches("info") {
        return execute_info(sub_matches);
    }
    unreachable!();
}

fn execute_info(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let trace_id = matches.get_one::<String>("trace_id").unwrap();

    let api = Api::current();
    let authenticated = api.authenticated()?;

    let meta = authenticated.get_trace_meta(&org, trace_id)?;
    let spans = authenticated.get_trace(&org, trace_id)?;

    println!("Trace: {}", trace_id);
    println!();
    println!("Summary:");
    println!("  Spans: {}", meta.span_count.unwrap_or(0));
    println!("  Errors: {}", meta.errors.unwrap_or(0));
    println!(
        "  Performance Issues: {}",
        meta.performance_issues.unwrap_or(0)
    );

    if !spans.is_empty() {
        println!();
        println!("Trace Tree:");
        for span in &spans {
            print_span(span, 0);
        }
    }

    Ok(())
}

fn print_span(span: &TraceSpan, depth: usize) {
    let indent = "  ".repeat(depth);
    let prefix = if depth == 0 { "-" } else { "|-" };

    let op = span.op.as_deref().unwrap_or("unknown");
    let desc = span.description.as_deref().unwrap_or("");
    let duration = span
        .duration
        .map(|d| format!(" ({:.0}ms)", d))
        .unwrap_or_default();

    let has_error = !span.errors.is_empty();
    let error_marker = if has_error { " [error]" } else { "" };

    println!(
        "{}{} [{}] {}{}{}",
        indent, prefix, op, desc, duration, error_marker
    );

    for child in &span.children {
        print_span(child, depth + 1);
    }
}
