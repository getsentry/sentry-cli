use std::fs;
use std::env;
use std::path::Path;
use std::io::{Read, Write};

use clap::{App, Arg, ArgMatches};
use hyper::client::{Client, RedirectPolicy};
use hyper::client::request::Request;
use hyper::header::{UserAgent, ContentLength};
use hyper::method::Method;
use url::Url;
use serde_json;
use runas;

use utils;
use CliResult;
use commands::Config;
use constants::{VERSION, PLATFORM, ARCH, EXT};

#[derive(Debug, Serialize, Deserialize)]
struct Asset {
    browser_download_url: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

struct ReleaseInfo {
    version: String,
    download_url: Url,
}


fn get_asset_name() -> CliResult<String> {
    Ok(format!("sentry-cli-{}-{}{}",
               utils::capitalize_string(PLATFORM), ARCH, EXT))
}

fn get_latest_release() -> CliResult<ReleaseInfo> {
    let ref_name = get_asset_name()?;

    let mut req = Request::new(Method::Get,
        Url::parse("https://api.github.com/repos/getsentry/sentry-cli/releases/latest")?)?;
    {
        let mut headers = req.headers_mut();
        headers.set(UserAgent("sentry-cli".into()));
    }
    let mut resp = req.start()?.send()?;
    if !resp.status.is_success() {
        fail!(resp);
    }

    let rv : Release = serde_json::from_reader(&mut resp)?;

    for asset in rv.assets {
        if asset.name == ref_name {
            return Ok(ReleaseInfo {
                version: rv.tag_name,
                download_url: Url::parse(&asset.browser_download_url)?
            });
        }
    }

    fail!("Could not find download URL for updates.");
}

fn download_url<P: AsRef<Path>>(url: &Url, dst: P) -> CliResult<()> {
    let mut client = Client::new();
    client.set_redirect_policy(RedirectPolicy::FollowAll);
    let mut resp = client.get(url.to_owned()).send()?;
    if !resp.status.is_success() {
        fail!(resp);
    }
    let content_length = match resp.headers.get::<ContentLength>() {
        Some(&ContentLength(length)) => length,
        None => 0
    };

    print!("Downloading...");
    let mut last_status = 0;
    let mut buffer = [0; 4096];
    let mut downloaded = 0;
    let mut f = fs::File::create(dst)?;
    loop {
        let chunk_size = resp.read(&mut buffer)?;
        if chunk_size == 0 {
            break;
        } else {
            f.write(&buffer[..chunk_size])?;
            downloaded += chunk_size as u64;
        }
        if content_length > 0 {
            let status = downloaded * 100 / content_length;
            if status != last_status {
                print!("\rDownloading... {}%", status);
                last_status = status;
            }
        }
    }
    println!("");
    println!("Done!");
    Ok(())
}

#[cfg(windows)]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> CliResult<()>
{
    // so on windows you can rename a running executable but you cannot delete it.
    // we move the old executable to a temporary location (this most likely only
    // works if they are on the same FS) and then put the new in place.  This
    // will leave the old executable in the temp path lying around so let's hope
    // that windows cleans up temp files there (spoiler: it does not)
    let tmp = env::temp_dir().join(".sentry-cli.tmp");

    if elevate {
        runas::Command::new("cmd")
            .arg("/c")
            .arg("move")
            .arg(&exe)
            .arg(&tmp)
            .arg("&")
            .arg("move")
            .arg(&downloaded_path)
            .arg(&exe)
            .arg("&")
            .arg("del")
            .arg(&tmp)
            .status()?;
    } else {
        fs::rename(&exe, &tmp)?;
        fs::rename(&downloaded_path, &exe)?;
        fs::remove_file(&tmp).ok();
    }

    Ok(())
}

#[cfg(not(windows))]
fn rename_exe(exe: &Path, downloaded_path: &Path, elevate: bool) -> CliResult<()>
{
    if elevate {
        println!("Need to sudo to overwrite {}", exe.display());
        runas::Command::new("mv")
            .arg(&downloaded_path)
            .arg(&exe)
            .status()?;
    } else {
        fs::rename(&downloaded_path, &exe)?;
    }
    Ok(())
}

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("update the sentry-cli executable")
        .arg(Arg::with_name("force")
             .long("force")
             .short("f")
             .help("Force the update even if already current."))
}

pub fn execute<'a>(matches: &ArgMatches<'a>, _config: &Config) -> CliResult<()> {
    let exe = env::current_exe()?;
    let elevate = !utils::is_writable(&exe);
    let latest_release = get_latest_release()?;
    let tmp_path = if elevate {
        env::temp_dir().join(".sentry-cli.part")
    } else {
        exe.parent().unwrap().join(".sentry-cli.part")
    };

    println!("Latest release is {}", latest_release.version);
    if latest_release.version == VERSION {
        if matches.is_present("force") {
            println!("Forcing update");
        } else {
            println!("Already up to date!");
            return Ok(());
        }
    }

    println!("Updating executable at {}", exe.display());

    match download_url(&latest_release.download_url, &tmp_path) {
        Err(err) => {
            fs::remove_file(tmp_path).ok();
            return Err(err);
        },
        Ok(()) => {},
    }

    utils::set_executable_mode(&tmp_path)?;

    rename_exe(&exe, &tmp_path, elevate)?;

    println!("Updated!");

    Ok(())
}
