use clap::{App, ArgMatches};

use api::Api;
use CliResult;
use commands::Config;

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("print out information about the sentry server")
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, config: &Config) -> CliResult<()> {
    let (project, org) = config.get_org_and_project_defaults();
    println!("Sentry Server:   {}", config.url);
    println!("Organization:    {}", project.unwrap_or("-".into()));
    println!("Project:         {}", org.unwrap_or("-".into()));
    println!("");

    println!("Authentication Info:");
    println!("  Method:        {}", config.auth.describe());
    match Api::new(config).get_auth_info() {
        Ok(info) => {
            if let Some(ref user) = info.user {
                println!("  User:          {} (id={})", user.email, user.id);
            }
            println!("  Scopes:");
            for scope in info.auth.scopes {
                println!("    * {}", scope);
            }
        },
        Err(err) => {
            println!("  (cannot auth: {})", err);
        }
    }
    Ok(())
}
