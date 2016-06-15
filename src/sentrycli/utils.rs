use std::io;
use std::fs;
use std::env;
use std::time;
use std::process;
use std::path::Path;
use std::io::{Read, Write, Seek};

use uuid::Uuid;
use sha1::Sha1;
use clap::{App, AppSettings};

use CliResult;

#[cfg(not(windows))]
use chan_signal::{notify, Signal};

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

#[cfg(not(windows))]
pub fn run_or_interrupt<F>(f: F) -> Option<Signal>
    where F: FnOnce() -> (), F: Send + 'static
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

pub fn make_subcommand<'a, 'b: 'a>(name: &str) -> App<'a, 'b> {
    App::new(name)
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::DisableVersion)
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

pub fn escape_win_shell_arg(arg) -> String {
    if arg.len() == 0 {
        "\"\""
    } else if arg.find(&[' ', '\t', '"']).is_none() {
        arg
    } else {
        let mut rv = String::with_capacity(rv.len() + 2);
        rv.push('"');
        for c in arg.chars() {
            match c {
                '\\' => rv.push("\\\\"),
                '"' => rv.push("\\\""),
                c => rv.push(c)
            }
        }
        rv.push('"');
        rv
    }
}

pub fn run_elevated(cmd: &str, args: &[&str]) -> io::Result<process::ExitStatus> {
    #[cfg(not(windows))]
    fn run(cmd: &str, args: &[&str]) -> io::Result<process::ExitStatus> {
        Command::new("sudo")
            .arg("-k")
            .arg(command)
            .args(args)
            .status()
    }
    #[cfg(windows)]
    fn run(cmd: &str, args: &[&str]) -> io::Result<process::ExitStatus> {
        use std::ptr::null;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use ole32::CoInitializeEx;
        use shell32::ShellExecuteW;
        use winapi::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        use winapi::winuser::SW_HIDE;

        pub mut params = String::new();
        for arg in args {
            params.push(' ');
            params.push(escape_shell_arg(arg));
        }

        let file = OsStr::new(cmd).encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let params = OsStr::new(params).encode_wide().chain(Some(0)).collect::<Vec<_>>();
        CoInitializeEx(NULL, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        ShellExecuteW(null(), "runas", &file, &params, null(), SW_HIDE);
    }

    run(cmd, args)
}

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

pub fn to_timestamp(tm: time::SystemTime) -> f64 {
    let duration = tm.duration_since(time::UNIX_EPOCH).unwrap();
    (duration.as_secs() as f64) + (duration.subsec_nanos() as f64 / 1e09)
}

pub fn capitalize_string(s: &str) -> String {
    use std::ascii::AsciiExt;
    let mut bytes = s.as_bytes().to_vec();
    bytes.make_ascii_lowercase();
    bytes[0] = bytes[0].to_ascii_uppercase();
    String::from_utf8(bytes).unwrap()
}
