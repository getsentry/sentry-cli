//! Various utility functionality.
use std::io;
use std::fs;
use std::mem;
use std::env;
use std::fmt;
use std::time;
use std::process;
use std::borrow::Cow;
use std::fmt::Display;
use std::result::Result as StdResult;
use std::path::{Path, PathBuf};
use std::io::{Read, Write, Seek, SeekFrom};

use clap;
use log;
use uuid::{Uuid, UuidVersion};
use sha1::Sha1;
use regex::{Regex, Captures};
use prettytable;
use chrono::{Duration, DateTime, UTC, TimeZone};
use indicatif::{style, Color, ProgressBar, ProgressStyle};

use prelude::*;

#[cfg(not(windows))]
use chan_signal::{notify, Signal};

/// Helper for formatting durations.
pub struct HumanDuration(pub Duration);

impl<'a> fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! try_write {
            ($num:expr, $str:expr) => {
                if $num == 1 { return write!(f, "1 {}", $str); }
                else if $num > 1 { return write!(f, "{} {}s", $num, $str); }
            }
        }

        try_write!(self.0.num_hours(), "hour");
        try_write!(self.0.num_minutes(), "minute");
        try_write!(self.0.num_seconds(), "second");
        write!(f, "0 seconds")
    }
}

/// A simple logger
pub struct Logger;

impl Logger {
    pub fn get_real_level(&self, metadata: &log::LogMetadata) -> log::LogLevel {
        // upgrade debug -> trace for mach_object as its too spammy otherwise
        if metadata.level() == log::LogLevel::Debug &&
           metadata.target().starts_with("mach_object::") {
            return log::LogLevel::Trace;
        }
        metadata.level()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        self.get_real_level(metadata) <= log::max_log_level()
    }

    fn log(&self, record: &log::LogRecord) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = self.get_real_level(record.metadata());
        let msg = format!("[{}] {} {}", level, record.target(), record.args());
        writeln!(io::stderr(), "{}", style(msg).fg(
            match level {
                log::LogLevel::Error | log::LogLevel::Warn => Color::Red,
                log::LogLevel::Info => Color::Cyan,
                log::LogLevel::Debug | log::LogLevel::Trace => Color::Yellow,
            }
        )).ok();
    }
}

pub struct Table {
    title_row: Option<TableRow>,
    rows: Vec<TableRow>,
}

pub struct TableRow {
    cells: Vec<prettytable::cell::Cell>,
}

impl TableRow {
    pub fn new() -> TableRow {
        TableRow {
            cells: vec![],
        }
    }

    pub fn add<D: Display>(&mut self, text: D) -> &mut TableRow {
        self.cells.push(prettytable::cell::Cell::new(&text.to_string()));
        self
    }

    fn make_row(&self) -> prettytable::row::Row {
        let mut row = prettytable::row::Row::empty();
        for cell in &self.cells {
            row.add_cell(cell.clone());
        }
        row
    }
}

impl Table {
    pub fn new() -> Table {
        Table {
            title_row: None,
            rows: vec![],
        }
    }

    pub fn title_row(&mut self) -> &mut TableRow {
        if self.title_row.is_none() {
            self.title_row = Some(TableRow::new());
        }
        self.title_row.as_mut().unwrap()
    }

    pub fn add_row(&mut self) -> &mut TableRow {
        self.rows.push(TableRow::new());
        let idx = self.rows.len() - 1;
        &mut self.rows[idx]
    }

    pub fn is_empty(&self) -> bool {
        self.rows.len() == 0
    }

    pub fn print(&self) {
        if self.is_empty() {
            return;
        }
        let mut tbl = prettytable::Table::new();
        tbl.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        if let Some(ref title_row) = self.title_row {
            tbl.set_titles(title_row.make_row());
        }
        for row in &self.rows {
            tbl.add_row(row.make_row());
        }
        tbl.print_tty(false);
    }
}

fn validate_org(v: String) -> StdResult<(), String> {
    if v.contains("/") || &v == "." || &v == ".." || v.contains(' ') {
        return Err("invalid value for organization. Use the URL \
                    slug and not the name!".into())
    } else {
        Ok(())
    }
}

pub fn validate_project(v: String) -> StdResult<(), String> {
    if v.contains("/") || &v == "." || &v == ".." || v.contains(' ') {
        return Err("invalid value for project. Use the URL \
                    slug and not the name!".into())
    } else {
        Ok(())
    }
}

fn validate_version(v: String) -> StdResult<(), String> {
    if v.len() == 0 || &v == "." || &v == ".." ||
       v.find(&['\n', '\t', '\x0b', '\x0c', '\t', '/'][..]).is_some() {
        Err(format!("Invalid release version. Slashes and certain \
                     whitespace characters are not permitted."))
    } else {
        Ok(())
    }
}

pub fn validate_seconds(v: String) -> StdResult<(), String> {
    if v.parse::<i64>().is_ok() {
        Ok(())
    } else {
        Err(format!("Invalid value (seconds as integer required)"))
    }
}

pub fn validate_timestamp(v: String) -> StdResult<(), String> {
    if let Err(err) = get_timestamp(&v) {
        Err(err.to_string())
    } else {
        Ok(())
    }
}

pub fn validate_uuid(s: String) -> StdResult<(), String> {
    if Uuid::parse_str(&s).is_err() {
        Err("Invalid UUID".to_string())
    } else {
        Ok(())
    }
}

pub fn get_timestamp(value: &str) -> Result<DateTime<UTC>> {
    if let Ok(int) = value.parse::<i64>() {
        Ok(UTC.timestamp(int, 0))
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        Ok(dt.with_timezone(&UTC))
    } else if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        Ok(dt.with_timezone(&UTC))
    } else {
        Err(Error::from("not in valid format. Unix timestamp or ISO 8601 date expected."))
    }
}

pub trait ArgExt: Sized {
    fn org_arg(self) -> Self;
    fn project_arg(self) -> Self;
    fn projects_arg(self) -> Self;
    fn org_project_args(self) -> Self {
        self.org_arg().project_arg()
    }
    fn version_arg(self, index: u64) -> Self;
}

impl<'a: 'b, 'b> ArgExt for clap::App<'a, 'b> {
    fn org_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("org")
            .value_name("ORG")
            .long("org")
            .short("o")
            .validator(validate_org)
            .help("The organization slug"))
    }

    fn project_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("project")
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .validator(validate_project)
            .help("The project slug"))
    }

    fn projects_arg(self) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("projects")
            .value_name("PROJECT")
            .long("project")
            .short("p")
            .multiple(true)
            .number_of_values(1)
            .required(false)
            .validator(validate_project)
            .help("The project slug.  This can be supplied multiple times."))
    }

    fn version_arg(self, index: u64) -> clap::App<'a, 'b> {
        self.arg(clap::Arg::with_name("version")
            .value_name("VERSION")
            .required(true)
            .index(index)
            .validator(validate_version)
            .help("The version of the release"))
    }
}

/// Helper for temporary file access
pub struct TempFile {
    f: Option<fs::File>,
    path: PathBuf,
}

impl TempFile {
    /// Creates a new tempfile.
    pub fn new() -> io::Result<TempFile> {
        let mut path = env::temp_dir();
        path.push(Uuid::new(UuidVersion::Random).unwrap().hyphenated().to_string());
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        Ok(TempFile {
            f: Some(f),
            path: path.to_path_buf(),
        })
    }

    /// Opens the tempfile
    pub fn open(&self) -> fs::File {
        let mut f = self.f.as_ref().unwrap().try_clone().unwrap();
        let _ = f.seek(SeekFrom::Start(0));
        f
    }

    /// Returns the path to the tempfile
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        mem::drop(self.f.take());
        let _ = fs::remove_file(&self.path);
    }
}

/// On non windows platforms this runs the function until it's
/// being interrupted by a signal.
#[cfg(not(windows))]
pub fn run_or_interrupt<F>(f: F) -> Option<Signal>
    where F: FnOnce() -> (),
          F: Send + 'static
{
    use chan;
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

/// Given a path returns the SHA1 checksum for it.
pub fn get_sha1_checksum<R: Read>(mut rdr: R) -> Result<String> {
    let mut sha = Sha1::new();
    let mut buf = [0u8; 16384];
    loop {
        let read = rdr.read(&mut buf)?;
        if read == 0 {
            break;
        }
        sha.update(&buf[..read]);
    }
    Ok(sha.digest().to_string())
}

/// Checks if a path is writable.
pub fn is_writable<P: AsRef<Path>>(path: P) -> bool {
    fs::OpenOptions::new().write(true).open(&path).map(|_| true).unwrap_or(false)
}

/// Set the mode of a path to 755 if we're on a Unix machine, otherwise
/// don't do anything with the given path.
pub fn set_executable_mode<P: AsRef<Path>>(path: P) -> io::Result<()> {
    #[cfg(not(windows))]
    fn exec<P: AsRef<Path>>(path: P) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = fs::metadata(&path)?.permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&path, perm)
    }

    #[cfg(windows)]
    fn exec<P: AsRef<Path>>(_path: P) -> io::Result<()> {
        Ok(())
    }

    exec(path)
}

/// Prints a message and loops until yes or no is entered.
pub fn prompt_to_continue(message: &str) -> io::Result<bool> {
    loop {
        print!("{} [y/n] ", message);
        io::stdout().flush()?;

        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim();

        if input == "y" {
            return Ok(true);
        } else if input == "n" {
            return Ok(false);
        }
        println!("invalid input!");
    }
}

/// Prompts for input and returns it.
pub fn prompt(message: &str) -> io::Result<String> {
    loop {
        print!("{}: ", message);
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim();
        if input.len() > 0 {
            return Ok(input.to_owned());
        }
    }
}

/// Given a system time returns the unix timestamp as f64
pub fn to_timestamp(tm: time::SystemTime) -> f64 {
    let duration = tm.duration_since(time::UNIX_EPOCH).unwrap();
    (duration.as_secs() as f64) + (duration.subsec_nanos() as f64 / 1e09)
}

/// Capitalizes a string and returns it.
pub fn capitalize_string(s: &str) -> String {
    use std::ascii::AsciiExt;
    let mut bytes = s.as_bytes().to_vec();
    bytes.make_ascii_lowercase();
    bytes[0] = bytes[0].to_ascii_uppercase();
    String::from_utf8(bytes).unwrap()
}

/// Checks if a file is a zip file and returns a result
pub fn is_zip_file_as_result<R: Read + Seek>(mut rdr: R) -> Result<bool> {
    let mut magic: [u8; 2] = [0; 2];
    rdr.read_exact(&mut magic)?;
    Ok(match &magic {
        b"PK" => true,
        _ => false
    })
}

/// Checks if a file is a zip file but only returns a bool
pub fn is_zip_file<R: Read + Seek>(rdr: R) -> bool {
    match is_zip_file_as_result(rdr) {
        Ok(val) => val,
        Err(_) => false,
    }
}

/// Propagate an exit status outwarts
pub fn propagate_exit_status(status: process::ExitStatus) {
    if !status.success() {
        if let Some(code) = status.code() {
            process::exit(code);
        } else {
            process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn is_homebrew_install_result() -> Result<bool> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.pop();
    exe.set_file_name("INSTALL_RECEIPT.json");
    Ok(exe.is_file())
}

#[cfg(windows)]
fn is_homebrew_install_result() -> Result<bool> {
    Ok(false)
}

fn is_npm_install_result() -> Result<bool> {
    let mut exe = env::current_exe()?.canonicalize()?;
    exe.pop();
    exe.set_file_name("package.json");
    Ok(exe.is_file())
}

/// Checks if we were installed from homebrew
pub fn is_homebrew_install() -> bool {
    is_homebrew_install_result().unwrap_or(false)
}

/// Checks if we were installed via npm
pub fn is_npm_install() -> bool {
    is_npm_install_result().unwrap_or(false)
}

/// Expands environment variables in a string
pub fn expand_envvars<'a>(s: &'a str) -> Cow<'a, str> {
    expand_vars(s, |key| {
        env::var(key).unwrap_or("".into())
    })
}

/// Expands variables in a string
pub fn expand_vars<'a, F: Fn(&str) -> String>(s: &'a str, f: F) -> Cow<'a, str> {
    lazy_static! {
        static ref VAR_RE: Regex = Regex::new(
            r"\$(\$|[a-zA-Z0-9_]+|\([^)]+\)|\{[^}]+\})").unwrap();
    }
    VAR_RE.replace_all(s, |caps: &Captures| {
        let key = &caps[1];
        if key == "$" {
            "$".into()
        } else if &key[..1] == "(" || &key[..1] == "{" {
            f(&key[1..key.len() - 1])
        } else {
            f(key)
        }
    })
}

/// Helper that renders an error to stderr.
pub fn print_error(err: &Error) {
    use std::error::Error;

    if let &ErrorKind::Clap(ref clap_err) = err.kind() {
        clap_err.exit();
    }

    writeln!(&mut io::stderr(), "error: {}", err).ok();
    let mut cause = err.cause();
    while let Some(the_cause) = cause {
        writeln!(&mut io::stderr(), "  caused by: {}", the_cause).ok();
        cause = the_cause.cause();
    }

    if env::var("RUST_BACKTRACE") == Ok("1".into()) {
        writeln!(&mut io::stderr(), "").ok();
        writeln!(&mut io::stderr(), "{:?}", err.backtrace()).ok();
    }
}

/// Like ``io::copy`` but advances a progress bar set to bytes.
pub fn copy_with_progress<R: ?Sized, W: ?Sized>(progress: &ProgressBar,
                                                reader: &mut R, writer: &mut W)
    -> io::Result<u64>
    where R: Read, W: Write
{
    let mut buf = [0; 16384];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        written += len as u64;
        progress.inc(len as u64);
    }
}

/// Creates a progress bar for byte stuff
pub fn make_byte_progress_bar(length: u64) -> ProgressBar {
    let pb = ProgressBar::new(length);
    pb.set_style(ProgressStyle::default_bar()
        .template("{wide_bar}  {bytes}/{total_bytes} ({eta})"));
    pb
}
