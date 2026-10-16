use std::collections::BTreeMap;
use std::io;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Serialize;

use crate::api::{outcome_label, Api, DroppedEventsBucket};
use crate::config::Config;
use crate::utils::formatting::Table;

const DATASETS: &[&str] = &["errors", "spans", "logs", "tracemetrics"];

pub fn make_command(command: Command) -> Command {
    command
        .about("Summarize events Sentry dropped, grouped by outcome and reason.")
        .arg(
            Arg::new("dataset")
                .long("dataset")
                .value_name("DATASET")
                .default_value("errors")
                .value_parser(DATASETS.to_vec())
                .help("Which dropped-data type to summarize."),
        )
        .arg(
            Arg::new("stats_period")
                .long("stats-period")
                .value_name("STATS_PERIOD")
                .default_value("90d")
                .help("Time range to summarize (e.g. 24h, 14d, 90d)."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output the summary as JSON (for agents/automation)."),
        )
}

#[derive(Serialize)]
struct ReasonRow {
    reason: String,
    category: String,
    events: u64,
    share: f64,
}

#[derive(Serialize)]
struct OutcomeSection {
    outcome: String,
    label: String,
    events: u64,
    share: f64,
    reasons: Vec<ReasonRow>,
}

fn share_ratio(events: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        events as f64 / total as f64
    }
}

fn format_share(ratio: f64) -> String {
    if ratio > 0.0 && ratio < 0.0001 {
        return "<0.01%".to_owned();
    }
    format!("{:.2}%", ratio * 100.0)
}

struct ReasonTotals {
    category: String,
    events: u64,
}

fn to_sections(dropped: &[DroppedEventsBucket], total: u64) -> Vec<OutcomeSection> {
    let mut by_outcome: BTreeMap<String, BTreeMap<String, ReasonTotals>> = BTreeMap::new();
    for bucket in dropped {
        let reasons = by_outcome.entry(bucket.outcome.clone()).or_default();
        let totals = reasons
            .entry(bucket.reason.clone())
            .or_insert_with(|| ReasonTotals {
                category: bucket.category.clone(),
                events: 0,
            });
        totals.events += bucket.count as u64;
    }

    let mut sections: Vec<OutcomeSection> = by_outcome
        .into_iter()
        .map(|(outcome, reasons)| {
            let mut reasons: Vec<ReasonRow> = reasons
                .into_iter()
                .map(|(reason, t)| ReasonRow {
                    reason,
                    category: t.category,
                    events: t.events,
                    share: share_ratio(t.events, total),
                })
                .collect();
            reasons.sort_by_key(|r| std::cmp::Reverse(r.events));
            let events = reasons.iter().map(|r| r.events).sum();
            OutcomeSection {
                label: outcome_label(&outcome).to_owned(),
                outcome,
                events,
                share: share_ratio(events, total),
                reasons,
            }
        })
        .collect();
    sections.sort_by_key(|s| std::cmp::Reverse(s.events));
    sections
}

pub fn execute(matches: &ArgMatches) -> Result<()> {
    let config = Config::current();
    let org = config.get_org(matches)?;
    let project = config.get_project(matches)?;
    #[expect(clippy::unwrap_used, reason = "clap supplies defaults")]
    let dataset = matches.get_one::<String>("dataset").unwrap();
    #[expect(clippy::unwrap_used, reason = "clap supplies defaults")]
    let stats_period = matches.get_one::<String>("stats_period").unwrap();

    let api = Api::current();
    let response = api.authenticated()?.fetch_organization_events_dropped(
        &org,
        &project,
        dataset,
        stats_period,
    )?;

    let total: u64 = response
        .dropped_events
        .iter()
        .chain(response.accepted_events.iter())
        .map(|b| b.count as u64)
        .sum();

    let sections = to_sections(&response.dropped_events, total);

    if matches.get_flag("json") {
        serde_json::to_writer_pretty(&mut io::stdout(), &sections)?;
        println!();
        return Ok(());
    }

    if sections.is_empty() {
        println!("No dropped events found");
        return Ok(());
    }

    for section in &sections {
        println!(
            "\n{}: {} dropped ({})",
            section.label,
            section.events,
            format_share(section.share),
        );

        let mut table = Table::new();
        table
            .title_row()
            .add("Reason")
            .add("Category")
            .add("Dropped")
            .add("Share");
        for row in &section.reasons {
            table
                .add_row()
                .add(&row.reason)
                .add(&row.category)
                .add(row.events)
                .add(format_share(row.share));
        }
        table.print();
    }

    Ok(())
}
