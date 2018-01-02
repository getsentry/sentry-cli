use std::fs;
use std::env;
use std::fmt;
use std::process;
use std::path::{Path, PathBuf};
use std::io::{BufReader, BufRead, Cursor};
use std::collections::HashMap;
#[cfg(target_os="macos")]
use libc::getpid;

use serde_json;
use plist::serde::deserialize;
#[cfg(target_os="macos")]
use osascript;
#[cfg(target_os="macos")]
use unix_daemonize::{daemonize_redirect, ChdirMode};
#[cfg(target_os="macos")]
use mac_process_info;
use regex::Regex;

use prelude::*;
use config::Config;
use utils::{TempFile, expand_vars, SeekRead};


#[derive(Deserialize, Debug)]
pub struct InfoPlist {
    #[serde(rename="CFBundleName")]
    name: String,
    #[serde(rename="CFBundleIdentifier")]
    bundle_id: String,
    #[serde(rename="CFBundleShortVersionString")]
    version: String,
    #[serde(rename="CFBundleVersion")]
    build: String,
}

#[derive(Deserialize, Debug)]
pub struct XcodeProjectInfo {
    targets: Vec<String>,
    schemes: Vec<String>,
    configurations: Vec<String>,
    name: String,
    #[serde(default="PathBuf::new")]
    path: PathBuf,
}

impl fmt::Display for InfoPlist {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name(), &self.version)
    }
}

pub fn expand_xcodevars(s: String, vars: &HashMap<String, String>) -> String {
    lazy_static! {
        static ref SEP_RE: Regex = Regex::new(r"[\s/]+").unwrap();
    }
    expand_vars(&s, |key| {
        if key == "" {
            return "".into();
        }
        let mut iter = key.splitn(2, ':');
        let value = vars.get(iter.next().unwrap()).map(|x| x.as_str()).unwrap_or("");
        match iter.next() {
            Some("rfc1034identifier") => {
                SEP_RE.replace_all(value, "-").into_owned()
            },
            Some("identifier") => {
                SEP_RE.replace_all(value, "_").into_owned()
            },
            None | Some(_) => value.to_string()
        }
    }).into_owned()
}

fn get_xcode_project_info(path: &Path) -> Result<Option<XcodeProjectInfo>> {
    if_chain! {
        if let Some(filename_os) = path.file_name();
        if let Some(filename) = filename_os.to_str();
        if filename.ends_with(".xcodeproj");
        then {
            return Ok(Some(XcodeProjectInfo::from_path(path)?));
        }
    }

    let mut projects = vec![];
    for entry_rv in fs::read_dir(path)? {
        if let Ok(entry) = entry_rv {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".xcodeproj") {
                    projects.push(entry.path().to_path_buf());
                }
            }
        }
    }

    if projects.len() == 1 {
        Ok(Some(XcodeProjectInfo::from_path(&projects[0])?))
    } else {
        Ok(None)
    }
}

impl XcodeProjectInfo {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<XcodeProjectInfo> {
        #[derive(Deserialize)]
        struct Output {
            project: XcodeProjectInfo,
        }
        let p = process::Command::new("xcodebuild")
            .arg("-list")
            .arg("-json")
            .arg("-project")
            .arg(path.as_ref().as_os_str())
            .output()?;
        let mut rv: Output = serde_json::from_slice(&p.stdout)?;
        rv.project.path = path.as_ref().canonicalize()?;
        Ok(rv.project)
    }

    pub fn base_path(&self) -> &Path {
        self.path.parent().unwrap()
    }

    pub fn get_build_vars(&self, target: &str, configuration: &str)
        -> Result<HashMap<String, String>>
    {
        let mut rv = HashMap::new();
        let p = process::Command::new("xcodebuild")
            .arg("-showBuildSettings")
            .arg("-project")
            .arg(&self.path)
            .arg("-target")
            .arg(target)
            .arg("-configuration")
            .arg(configuration)
            .output()?;
        for line_rv in p.stdout.lines() {
            let line = line_rv?;
            if line.starts_with("    ") {
                let mut sep = line[4..].splitn(2, " = ");
                if_chain! {
                    if let Some(key) = sep.next();
                    if let Some(value) = sep.next();
                    then {
                        rv.insert(key.to_owned(), value.to_owned());
                    }
                }
            }
        }
        Ok(rv)
    }

    /// Return the first target
    pub fn get_first_target(&self) -> Option<&str> {
        if !self.targets.is_empty() {
            Some(&self.targets[0])
        } else {
            None
        }
    }

    /// Returns the config with a certain name
    pub fn get_configuration(&self, name: &str) -> Option<&str> {
        let name = name.to_lowercase();
        for cfg in &self.configurations {
            if cfg.to_lowercase() == name {
                return Some(&cfg);
            }
        }
        None
    }
}

impl InfoPlist {

    /// Loads a processed plist file.
    pub fn discover_from_env() -> Result<Option<InfoPlist>> {
        // if we are loaded directly from xcode we can trust the os environment
        // and pass those variables to the processor.
        if env::var("XCODE_VERSION_ACTUAL").is_ok() {
            match (
                env::var("BUILT_PRODUCTS_DIR"),
                env::var("INFOPLIST_PATH")
            ) {
                (Ok(dir), Ok(filepath)) => {
                    let path: PathBuf = [dir, filepath].iter().collect();
                    let vars: HashMap<_, _> = env::vars().collect();
                    Ok(Some(InfoPlist::load_and_process(&path, &vars)?))
                }
                _ => Ok(None)
            }

        // otherwise, we discover the project info from the current path and
        // invoke xcodebuild to give us the project settings for the first
        // target.
        } else {
            if_chain! {
                if let Ok(here) = env::current_dir();
                if let Some(pi) = get_xcode_project_info(&here)?;
                then {
                    InfoPlist::from_project_info(&pi)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Lodas an info plist from a given project info
    pub fn from_project_info(pi: &XcodeProjectInfo) -> Result<Option<InfoPlist>> {
        if_chain! {
            if let Some(config) = pi.get_configuration("release")
                .or_else(|| pi.get_configuration("debug"));
            if let Some(target) = pi.get_first_target();
            then {
                let vars = pi.get_build_vars(target, config)?;
                if let Some(path) = vars.get("INFOPLIST_FILE") {
                    let base = vars.get("PROJECT_DIR").map(|x| Path::new(x.as_str()))
                        .unwrap_or(pi.base_path());
                    let path = base.join(path);
                    return Ok(Some(InfoPlist::load_and_process(path, &vars)?));
                }
            }
        }
        Ok(None)
    }

    /// loads an info plist file from a path and processes it with the given vars
    pub fn load_and_process<P: AsRef<Path>>(path: P, vars: &HashMap<String, String>)
        -> Result<InfoPlist>
    {
        // do we want to preprocess the plist file?
        let mut rv = if vars.get("INFOPLIST_PREPROCESS").map(|x| x.as_str()) == Some("YES") {
            let mut c = process::Command::new("cc");
            c.arg("-xc")
                .arg("-P")
                .arg("-E");
            if let Some(defs) = vars.get("INFOPLIST_PREPROCESSOR_DEFINITIONS") {
                for token in defs.split_whitespace() {
                    c.arg(format!("-D{}", token));
                }
            }
            c.arg(path.as_ref());
            let p = c.output()?;
            InfoPlist::from_reader(&mut Cursor::new(&p.stdout[..]))?
        } else {
            InfoPlist::from_path(path)?
        };

        // expand xcodevars here
        rv.name = expand_xcodevars(rv.name, &vars);
        rv.bundle_id = expand_xcodevars(rv.bundle_id, &vars);
        rv.version = expand_xcodevars(rv.version, &vars);
        rv.build = expand_xcodevars(rv.build, &vars);

        Ok(rv)
    }

    /// Loads an info plist file from a path and does not process it.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<InfoPlist> {
        let mut f = fs::File::open(path.as_ref()).chain_err(||
            Error::from("Could not open Info.plist file"))?;
        InfoPlist::from_reader(&mut f)
    }

    /// Loads an info plist file from a reader.
    pub fn from_reader<R: SeekRead>(rdr: R) -> Result<InfoPlist> {
        let mut rdr = BufReader::new(rdr);
        Ok(deserialize(&mut rdr).chain_err(||
            Error::from("Could not parse Info.plist file"))?)
    }

    pub fn get_release_name(&self) -> String {
        format!("{}-{}", self.bundle_id(), self.version())
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn build(&self) -> &str {
        &self.build
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bundle_id(&self) -> &str {
        &self.bundle_id
    }
}

/// Helper struct that allows the current execution to detach from
/// the xcode console and continue in the background.  This becomes
/// a dummy shim for non xcode runs or platforms.
pub struct MayDetach<'a> {
    #[allow(dead_code)]
    config: &'a Config,
    output_file: Option<TempFile>,
    #[allow(dead_code)]
    task_name: &'a str,
}

impl<'a> MayDetach<'a> {
    fn new(config: &'a Config, task_name: &'a str) -> MayDetach<'a> {
        MayDetach {
            config: config,
            output_file: None,
            task_name: task_name,
        }
    }

    /// Returns true if we are deteached from xcode
    pub fn is_detached(&self) -> bool {
        self.output_file.is_some()
    }

    /// If we are launched from xcode this detaches us from the xcode console
    /// and continues execution in the background.  From this moment on output
    /// is captured and the user is notified with notifications.
    #[cfg(target_os="macos")]
    pub fn may_detach(&mut self) -> Result<bool> {
        if !launched_from_xcode() {
            return Ok(false);
        }

        println!("Continuing in background.");
        show_notification(self.config, "Sentry", &format!("{} starting", self.task_name))?;
        let output_file = TempFile::new()?;
        daemonize_redirect(Some(output_file.path()),
                           Some(output_file.path()),
                           ChdirMode::NoChdir).unwrap();
        self.output_file = Some(output_file);
        Ok(true)
    }

    /// For non mac platforms this just never detaches.
    #[cfg(not(target_os="macos"))]
    pub fn may_detach(&mut self) -> Result<bool> {
        Ok(false)
    }

    /// Wraps the execution of a code block.  Does not detach until someone
    /// calls into `may_detach`.
    #[cfg(target_os="macos")]
    pub fn wrap<T, F: FnOnce(&mut MayDetach) -> Result<T>>(
        config: &Config, task_name: &'a str, f: F) -> Result<T>
    {
        use std::time::Duration;
        use std::thread;
        use open;
        use utils::print_error;

        let mut md = MayDetach::new(config, task_name);
        match f(&mut md) {
            Ok(x) => {
                md.show_done()?;
                Ok(x)
            }
            Err(err) => {
                if let Some(ref output_file) = md.output_file {
                    print_error(&err);
                    if md.show_critical_info()? {
                        open::that(&output_file.path())?;
                        thread::sleep(Duration::from_millis(5000));
                    }
                }
                Err(err)
            }
        }
    }

    /// Dummy wrap call that never detaches for non mac platforms.
    #[cfg(not(target_os="macos"))]
    pub fn wrap<T, F: FnOnce(&mut MayDetach) -> Result<T>>(
        config: &Config, task_name: &'a str, f: F) -> Result<T> {
        f(&mut MayDetach::new(config, task_name))
    }

    #[cfg(target_os="macos")]
    fn show_critical_info(&self) -> Result<bool> {
        show_critical_info(
            &format!("{} failed", self.task_name),
            "The Sentry build step failed while running in the background. \
             You can ignore this error or view details to attempt to resolve \
             it. Ignoring it might cause your crashes not to be handled \
             properly.")
    }

    #[cfg(target_os="macos")]
    fn show_done(&self) -> Result<()> {
        if self.is_detached() {
            show_notification(self.config, "Sentry", &format!("{} finished", self.task_name))?;
        }
        Ok(())
    }
}

/// Returns true if we were invoked from xcode
#[cfg(target_os="macos")]
pub fn launched_from_xcode() -> bool {
    if env::var("XCODE_VERSION_ACTUAL").is_err() {
        return false;
    }

    let mut pid = unsafe { getpid() as u32 };
    while let Some(parent) = mac_process_info::get_parent_pid(pid) {
        if parent == 1 {
            break;
        }
        if let Ok(name) = mac_process_info::get_process_name(parent) {
            if &name == "Xcode" {
                return true;
            }
        }
        pid = parent;
    }

    false
}

/// Shows a dialog in xcode and blocks.  The dialog will have a title and a
/// message as well as the buttons "Show details" and "Ignore".  Returns
/// `true` if the `show details` button has been pressed.
#[cfg(target_os="macos")]
pub fn show_critical_info(title: &str, msg: &str) -> Result<bool> {
    lazy_static! {
        static ref SCRIPT: osascript::JavaScript = osascript::JavaScript::new("
            var App = Application('XCode');
            App.includeStandardAdditions = true;
            return App.displayAlert($params.title, {
                message: $params.message,
                as: \"critical\",
                buttons: [\"Show details\", \"Ignore\"]
            });
        ");
    }

    #[derive(Serialize)]
    struct AlertParams<'a> {
        title: &'a str,
        message: &'a str,
    }

    #[derive(Debug, Deserialize)]
    struct AlertResult {
        #[serde(rename="buttonReturned")]
        button: String,
    }

    let rv: AlertResult = SCRIPT.execute_with_params(AlertParams {
        title: title,
        message: msg,
    }).chain_err(|| "Failed to display Xcode dialog")?;

    Ok(&rv.button != "Ignore")
}

/// Shows a notification in xcode
#[cfg(target_os="macos")]
pub fn show_notification(config: &Config, title: &str, msg: &str) -> Result<()> {
    lazy_static! {
        static ref SCRIPT: osascript::JavaScript = osascript::JavaScript::new("
            var App = Application.currentApplication();
            App.includeStandardAdditions = true;
            App.displayNotification($params.message, {
                withTitle: $params.title
            });
        ");
    }

    if !config.show_notifications()? {
        return Ok(());
    }

    #[derive(Serialize)]
    struct NotificationParams<'a> {
        title: &'a str,
        message: &'a str,
    }

    Ok(SCRIPT.execute_with_params(NotificationParams {
        title: title,
        message: msg,
    }).chain_err(|| "Failed to display Xcode notification")?)
}

#[test]
fn test_expansion() {
    let mut vars = HashMap::new();
    vars.insert("FOO_BAR".to_string(), "foo bar baz / blah".to_string());

    assert_eq!(expand_xcodevars("A$(FOO_BAR:rfc1034identifier)B".to_string(), &vars),
               "Afoo-bar-baz-blahB".to_string());
    assert_eq!(expand_xcodevars("A$(FOO_BAR:identifier)B".to_string(), &vars),
               "Afoo_bar_baz_blahB".to_string());
    assert_eq!(expand_xcodevars("A${FOO_BAR:identifier}B".to_string(), &vars),
               "Afoo_bar_baz_blahB".to_string());
}
