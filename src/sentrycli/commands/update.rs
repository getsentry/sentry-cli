use std::fs;
use std::env;
use std::path::Path;
use std::process::Command;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;

use clap::{App, ArgMatches};
use hyper::client::{Client, RedirectPolicy};
use hyper::client::request::Request;
use hyper::header::{UserAgent, ContentLength};
use hyper::method::Method;
use url::Url;
use serde_json;

use utils;
use CliResult;
use commands::Config;

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
    let p = Command::new("uname").arg("-sm").output()?;
    let output = String::from_utf8(p.stdout)?;
    let mut iter = output.trim().split(' ');
    let platform = iter.next().unwrap_or("unknown");
    let arch = iter.next().unwrap_or("unknown");
    Ok(format!("sentry-cli-{}-{}", platform, arch))
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

pub fn make_app<'a, 'b: 'a>(app: App<'a, 'b>) -> App<'a, 'b>
{
    app
        .about("update the sentry-cli executable")
}

pub fn execute<'a>(_matches: &ArgMatches<'a>, _config: &Config) -> CliResult<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let exe = env::current_exe()?;
    let need_sudo = !utils::is_writable(&exe);
    let latest_release = get_latest_release()?;
    let tmp_path = if need_sudo {
        env::temp_dir().join(".sentry-cli.part")
    } else {
        exe.parent().unwrap().join(".sentry-cli.part")
    };

    println!("Latest release is {}", latest_release.version);
    if latest_release.version == current_version {
        println!("Already up to date!");
        return Ok(());
    }

    println!("Updating executable at {}", exe.display());

    match download_url(&latest_release.download_url, &tmp_path) {
        Err(err) => {
            fs::remove_file(tmp_path).ok();
            return Err(err);
        },
        Ok(()) => {},
    }

    let mut perm = fs::metadata(&tmp_path)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&tmp_path, perm)?;

    if need_sudo {
        println!("Need to sudo to overwrite {}", exe.display());
        Command::new("sudo")
            .arg("-k")
            .arg("mv")
            .arg(&tmp_path)
            .arg(&exe)
            .status()?;
    } else {
        fs::rename(&tmp_path, &exe)?;
    }
    println!("Updated!");

    Ok(())
}
