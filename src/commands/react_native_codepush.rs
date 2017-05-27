use clap::{App, Arg, ArgMatches};

use prelude::*;
use api::Api;
use config::Config;
use utils::ArgExt;
use codepush::{get_codepush_package, get_codepush_release};

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.about("uploads react-native projects for codepush")
        .org_project_args()
        .arg(Arg::with_name("deployment")
            .long("deployment")
            .value_name("DEPLOYMENT")
            .help("The name of the deployment (Production, Staging)"))
        .arg(Arg::with_name("app_name")
            .value_name("APP_NAME")
            .index(1)
            .required(true)
            .help("The name of the code-push application"))
        .arg(Arg::with_name("platform")
            .value_name("PLATFORM")
            .index(2)
            .required(true)
            .help("The name of the code-push platform (ios, android)"))
        .arg(Arg::with_name("paths")
            .value_name("PATH")
            .index(3)
            .required(true)
            .multiple(true)
            .help("A list of folders with assets that should be processed."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, config: &Config) -> Result<()> {
    let (org, project) = config.get_org_and_project(matches)?;
    let app = matches.value_of("app_name").unwrap();
    let platform = matches.value_of("platform").unwrap();
    let deployment = matches.value_of("deployment").unwrap_or("Staging");
    let api = Api::new(config);

    let package = get_codepush_package(app, deployment)?;
    let release = get_codepush_release(&package, platform)?;

    println!("release is {}", release);

    Ok(())
}
