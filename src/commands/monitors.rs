//! Implements a command for managing projects.
use clap::{App, AppSettings, Arg, ArgMatches};
use failure::Error;
use std::rc::Rc;
use subprocess::{Popen, PopenConfig};

use crate::api::{CreateMonitorCheckIn, UpdateMonitorCheckIn, Api};
use crate::config::Config;
use crate::utils::args::ArgExt;
use crate::utils::formatting::Table;

struct MonitorContext {
    pub api: Rc<Api>,
    pub org: String,
}

impl<'a> MonitorContext {
    pub fn get_org(&'a self) -> Result<&str, Error> {
        Ok(&self.org)
    }
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("Manage monitors on Sentry.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .org_arg()
        .subcommand(App::new("list").about("List all monitors for an organization."))
        .subcommand(App::new("checkin")
            .arg(Arg::with_name("monitor")
                .help("The monitor ID")
                .required(true)
                .index(1))
            .arg(Arg::with_name("args")
                .required(true)
                .multiple(true)
                .last(true)))
}

pub fn execute<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let config = Config::get_current();

    let ctx = MonitorContext {
        api: Api::get_current(),
        org: config.get_org(matches)?,
    };

    if let Some(sub_matches) = matches.subcommand_matches("list") {
        return execute_list(&ctx, sub_matches);
    }
    if let Some(sub_matches) = matches.subcommand_matches("checkin") {
        return execute_checkin(&ctx, sub_matches);
    }
    unreachable!();
}

fn execute_list<'a>(ctx: &MonitorContext, _matches: &ArgMatches<'a>) -> Result<(), Error> {
    let mut monitors = ctx.api.list_organization_monitors(ctx.get_org()?)?;
    monitors.sort_by_key(|p| (p.name.clone()));

    let mut table = Table::new();
    table
        .title_row()
        .add("ID")
        .add("Name")
        .add("Status");

    for monitor in &monitors {
        table
            .add_row()
            .add(&monitor.id)
            .add(&monitor.name)
            .add(&monitor.status);
    }

    table.print();

    Ok(())
}

fn execute_checkin<'a>(ctx: &MonitorContext, matches: &ArgMatches<'a>) -> Result<(), Error> {
    // [cmd] checkin [monitor guid] [raw args]
    // if raw args == "foo --bar"

    let monitor = matches.value_of("monitor").unwrap();
    let args: Vec<_> = matches.values_of("args").unwrap().collect();

    // TODO(dcramer): does it automatically pass as a reference?
    let checkin = ctx.api.create_monitor_checkin(monitor, &CreateMonitorCheckIn {
        status: "in_progress".to_string(),
    })?;

    // TODO(dcramer):
    // - is this doing passthru on stdout/err
    // - what about the shell/env?
    let mut p = Popen::create(&args, PopenConfig::default())?;

    let exit_status = p.wait()?;

    let mut status = "";
    if exit_status.success() {
        status = "ok";
    } else {
        status = "error";
    }

    // write the result
    ctx.api.update_monitor_checkin(monitor, &checkin.id, &UpdateMonitorCheckIn {
        status: Some(status.to_string()),
        duration: Some(0),
    })?;

    Ok(())
}
