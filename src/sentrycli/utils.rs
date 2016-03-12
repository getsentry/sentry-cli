use std::io;
use std::fs;
use std::env;
use std::path::Path;
use std::io::{Read, Seek};

use uuid::Uuid;
use chan;
use sha1::Sha1;
use chan_signal::{notify, Signal};
use clap::{App, AppSettings, ArgMatches};

use CliResult;

pub struct TempFile {
    f: fs::File,
}

impl TempFile {
    pub fn new() -> io::Result<TempFile> {
        let mut path = env::temp_dir();
        path.push(Uuid::new_v4().to_hyphenated_string());
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path).unwrap();
        let _ = fs::remove_file(&path);
        Ok(TempFile {
            f: f
        })
    }

    pub fn open(&self) -> fs::File {
        let mut f = self.f.try_clone().unwrap();
        let _ = f.seek(io::SeekFrom::Start(0));
        f
    }
}

pub fn run_or_interrupt<F>(f: F) -> Option<Signal>
    where F: FnOnce() -> (), F: Send + 'static
{
    let run = |_sdone: chan::Sender<()>| f();
    let signal = notify(&[Signal::INT, Signal::TERM]);
    let (sdone, rdone) = chan::sync(0);
    ::std::thread::spawn(move || run(sdone));

    let mut rv = None;

    chan_select! {
        signal.recv() -> signal => { rv = signal; },
        rdone.recv() => {}
    }

    rv
}

pub fn make_subcommand<'a, 'b: 'a>(name: &str) -> App<'a, 'b> {
    App::new(name).setting(AppSettings::UnifiedHelpMessage)
}

pub fn get_org_and_project(matches: &ArgMatches) -> CliResult<(String, String)> {
    Ok((
        matches
            .value_of("org").map(|x| x.to_owned())
            .or_else(|| env::var("SENTRY_ORG").ok())
            .ok_or("An organization slug is required (provide with --org)")?,
        matches
            .value_of("project").map(|x| x.to_owned())
            .or_else(|| env::var("SENTRY_PROJECT").ok())
            .ok_or("A project slug is required (provide with --project)")?
    ))
}

pub fn get_sha1_checksum(path: &Path) -> CliResult<String> {
    let mut sha = Sha1::new();
    let mut f = fs::File::open(path)?;
    let mut buf = [0u8; 16384];
    loop {
        let read = f.read(&mut buf)?;
        if read == 0 {
            break;
        }
        sha.update(&buf[..read]);
    }
    Ok(sha.hexdigest())
}
