use clap::{App, ArgMatches};
use hyper::method::Method;
use serde_json;

use CliResult;
use commands::Config;

#[derive(Deserialize)]
struct Auth {
    scopes: Vec<String>,
}

#[derive(Deserialize)]
struct User {
    email: String,
    id: String,
}

#[derive(Deserialize)]
struct AuthInfo {
    auth: Auth,
    user: Option<User>,
}

fn get_user_info(config: &Config) -> CliResult<AuthInfo> {
    let mut resp = config.api_request(Method::Get, "/")?;
    if !resp.status.is_success() {
        fail!(resp);
    } else {
        Ok(serde_json::from_reader(&mut resp)?)
    }
}

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
    match get_user_info(&config) {
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
