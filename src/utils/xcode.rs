use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::hash::BuildHasher;
use std::io::{BufRead, BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{format_err, Context, Result};
use if_chain::if_chain;
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use serde::Deserialize;

#[cfg(target_os = "macos")]
use {
    libc::getpid,
    mac_process_info, osascript,
    unix_daemonize::{daemonize_redirect, ChdirMode},
};

use crate::utils::fs::{SeekRead, TempFile};
use crate::utils::system::expand_vars;

#[derive(Deserialize, Debug)]
pub struct InfoPlist {
    #[serde(rename = "CFBundleName")]
    name: String,
    #[serde(rename = "CFBundleIdentifier")]
    bundle_id: String,
    #[serde(rename = "CFBundleShortVersionString")]
    version: String,
    #[serde(rename = "CFBundleVersion")]
    build: String,
}

#[derive(Deserialize, Debug)]
pub struct XcodeProjectInfo {
    targets: Vec<String>,
    configurations: Vec<String>,
    #[serde(default = "PathBuf::new")]
    path: PathBuf,
}

impl fmt::Display for InfoPlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), &self.version)
    }
}

pub fn expand_xcodevars<S>(s: &str, vars: &HashMap<String, String, S>) -> String
where
    S: BuildHasher,
{
    lazy_static! {
        static ref SEP_RE: Regex = Regex::new(r"[\s/]+").unwrap();
    }

    expand_vars(s, |key| {
        if key.is_empty() {
            return "".into();
        }

        let mut iter = key.splitn(2, ':');
        let value = vars
            .get(iter.next().unwrap())
            .map(String::as_str)
            .unwrap_or("");

        match iter.next() {
            Some("rfc1034identifier") => SEP_RE.replace_all(value, "-").into_owned(),
            Some("identifier") => SEP_RE.replace_all(value, "_").into_owned(),
            None | Some(_) => value.to_string(),
        }
    })
    .into_owned()
}

fn get_xcode_project_info(path: &Path) -> Result<Option<XcodeProjectInfo>> {
    if_chain! {
        if let Some(filename_os) = path.file_name();
        if let Some(filename) = filename_os.to_str();
        if filename.ends_with(".xcodeproj");
        then {
            return match XcodeProjectInfo::from_path(path) {
                Ok(info) => Ok(Some(info)),
                _ => Ok(None),
            };
        }
    }

    let mut projects = vec![];
    for entry in (fs::read_dir(path)?).flatten() {
        if let Some(filename) = entry.file_name().to_str() {
            if filename.ends_with(".xcodeproj") {
                projects.push(entry.path().to_path_buf());
            }
        }
    }

    if projects.len() == 1 {
        match XcodeProjectInfo::from_path(&projects[0]) {
            Ok(info) => Ok(Some(info)),
            _ => Ok(None),
        }
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

        match serde_json::from_slice::<Output>(&p.stdout) {
            Ok(mut rv) => {
                rv.project.path = path.as_ref().canonicalize()?;
                Ok(rv.project)
            }
            Err(e) => {
                warn!("Your .xcodeproj might be malformed. Command `xcodebuild -list -json -project {}` failed to produce a valid JSON output.", path.as_ref().display());
                Err(e.into())
            }
        }
    }

    pub fn base_path(&self) -> &Path {
        self.path.parent().unwrap()
    }

    pub fn get_build_vars(
        &self,
        target: &str,
        configuration: &str,
    ) -> Result<HashMap<String, String>> {
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
            if let Some(suffix) = line.strip_prefix("    ") {
                let mut sep = suffix.splitn(2, " = ");
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
        self.configurations
            .iter()
            .find(|&cfg| cfg.to_lowercase() == name)
            .map(|v| v.as_ref())
    }
}

impl InfoPlist {
    /// Loads a processed plist file.
    pub fn discover_from_env() -> Result<Option<InfoPlist>> {
        // if we are loaded directly from xcode we can trust the os environment
        // and pass those variables to the processor.
        if env::var("XCODE_VERSION_ACTUAL").is_ok() {
            let vars: HashMap<_, _> = env::vars().collect();
            if let Some(filename) = vars.get("INFOPLIST_FILE") {
                let base = vars.get("PROJECT_DIR").map(String::as_str).unwrap_or(".");
                let path = env::current_dir().unwrap().join(base).join(filename);
                Ok(Some(InfoPlist::load_and_process(path, &vars)?))
            } else if let Ok(default_plist) = InfoPlist::from_env_vars(&vars) {
                Ok(Some(default_plist))
            } else {
                Ok(None)
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

    /// Loads an info plist from a given project info
    pub fn from_project_info(pi: &XcodeProjectInfo) -> Result<Option<InfoPlist>> {
        if_chain! {
            if let Some(config) = pi.get_configuration("release")
                .or_else(|| pi.get_configuration("debug"));
            if let Some(target) = pi.get_first_target();

            then {
                let vars = pi.get_build_vars(target, config)?;

                if let Some(path) = vars.get("INFOPLIST_FILE") {
                    let base = vars.get("PROJECT_DIR").map(Path::new)
                        .unwrap_or_else(|| pi.base_path());
                    let path = base.join(path);
                    return Ok(Some(InfoPlist::load_and_process(path, &vars)?))
                }
            }
        }
        Ok(None)
    }

    /// Loads an info plist file from a path and processes it with the given vars
    pub fn load_and_process<P: AsRef<Path>>(
        path: P,
        vars: &HashMap<String, String>,
    ) -> Result<InfoPlist> {
        // do we want to preprocess the plist file?
        let plist = if vars.get("INFOPLIST_PREPROCESS").map(String::as_str) == Some("YES") {
            let mut c = process::Command::new("cc");
            c.arg("-xc").arg("-P").arg("-E");
            if let Some(defs) = vars.get("INFOPLIST_OTHER_PREPROCESSOR_FLAGS") {
                for token in defs.split_whitespace() {
                    c.arg(token);
                }
            }
            if let Some(defs) = vars.get("INFOPLIST_PREPROCESSOR_DEFINITIONS") {
                for token in defs.split_whitespace() {
                    c.arg(format!("-D{token}"));
                }
            }
            c.arg(path.as_ref());
            let p = c.output()?;
            InfoPlist::from_reader(Cursor::new(&p.stdout[..]))
        } else {
            InfoPlist::from_path(path).or_else(|err| {
                /*
                This is sort of an edge-case, as XCode is not producing an `Info.plist` file
                by default anymore. However, it still does so for some templates.

                For example iOS Storyboard template will produce a partial `Info.plist` file,
                with a content only related to the Storyboard itself, but not the project as a whole. eg.

                <?xml version="1.0" encoding="UTF-8"?>
                <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
                <plist version="1.0">
                <dict>
                    <key>UIApplicationSceneManifest</key>
                    <dict>
                        <key>UISceneConfigurations</key>
                        <dict>
                            <key>UIWindowSceneSessionRoleApplication</key>
                            <array>
                                <dict>
                                    <key>UISceneStoryboardFile</key>
                                    <string>Main</string>
                                </dict>
                            </array>
                        </dict>
                    </dict>
                </dict>
                </plist>

                This causes a sort of false-positive, as `INFOPLIST_FILE` is present, yet it contains
                no data required by the CLI to correctly produce a `InfoPlist` struct.

                In the case like that, we try to fallback to env variables collected either by `xcodebuild` binary,
                or directly through `env` if we were called from within XCode itself.
                */
                InfoPlist::from_env_vars(vars).map_err(|e| e.context(err))
            })
        };

        plist.map(|raw| InfoPlist {
            name: expand_xcodevars(&raw.name, vars),
            bundle_id: expand_xcodevars(&raw.bundle_id, vars),
            version: expand_xcodevars(&raw.version, vars),
            build: expand_xcodevars(&raw.build, vars),
        })
    }

    /// Loads an info plist from provided environment variables list
    pub fn from_env_vars(vars: &HashMap<String, String>) -> Result<InfoPlist> {
        let name = vars
            .get("PRODUCT_NAME")
            .map(String::to_owned)
            .ok_or_else(|| format_err!("PRODUCT_NAME is missing"))?;
        let bundle_id = vars
            .get("PRODUCT_BUNDLE_IDENTIFIER")
            .map(String::to_owned)
            .ok_or_else(|| format_err!("PRODUCT_BUNDLE_IDENTIFIER is missing"))?;
        let version = vars
            .get("MARKETING_VERSION")
            .map(String::to_owned)
            .ok_or_else(|| format_err!("MARKETING_VERSION is missing"))?;
        let build = vars
            .get("CURRENT_PROJECT_VERSION")
            .map(String::to_owned)
            .ok_or_else(|| format_err!("CURRENT_PROJECT_VERSION is missing"))?;

        Ok(InfoPlist {
            name,
            bundle_id,
            version,
            build,
        })
    }

    /// Loads an info plist file from a path and does not process it.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<InfoPlist> {
        let mut f = fs::File::open(path.as_ref()).context("Could not open Info.plist file")?;
        InfoPlist::from_reader(&mut f)
    }

    /// Loads an info plist file from a reader.
    pub fn from_reader<R: SeekRead>(rdr: R) -> Result<InfoPlist> {
        let rdr = BufReader::new(rdr);
        plist::from_reader(rdr).context("Could not parse Info.plist file")
    }

    pub fn get_release_name(&self) -> String {
        format!("{}@{}", self.bundle_id(), self.version())
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
    output_file: Option<TempFile>,
    #[allow(dead_code)]
    task_name: &'a str,
}

impl<'a> MayDetach<'a> {
    fn new(task_name: &'a str) -> MayDetach<'a> {
        MayDetach {
            output_file: None,
            task_name,
        }
    }

    /// Returns true if we are deteached from xcode
    pub fn is_detached(&self) -> bool {
        self.output_file.is_some()
    }

    /// If we are launched from xcode this detaches us from the xcode console
    /// and continues execution in the background.  From this moment on output
    /// is captured and the user is notified with notifications.
    #[cfg(target_os = "macos")]
    pub fn may_detach(&mut self) -> Result<bool> {
        if !launched_from_xcode() {
            return Ok(false);
        }

        println!("Continuing in background.");
        show_notification("Sentry", &format!("{} starting", self.task_name))?;
        let output_file = TempFile::create()?;
        daemonize_redirect(
            Some(output_file.path()),
            Some(output_file.path()),
            ChdirMode::NoChdir,
        )
        .unwrap();
        self.output_file = Some(output_file);
        Ok(true)
    }

    /// For non mac platforms this just never detaches.
    #[cfg(not(target_os = "macos"))]
    pub fn may_detach(&mut self) -> Result<bool> {
        Ok(false)
    }

    /// Wraps the execution of a code block.  Does not detach until someone
    /// calls into `may_detach`.
    #[cfg(target_os = "macos")]
    pub fn wrap<T, F: FnOnce(&mut MayDetach<'_>) -> Result<T>>(
        task_name: &'a str,
        f: F,
    ) -> Result<T> {
        use std::time::Duration;

        let mut md = MayDetach::new(task_name);
        match f(&mut md) {
            Ok(x) => {
                md.show_done()?;
                Ok(x)
            }
            Err(err) => {
                if let Some(ref output_file) = md.output_file {
                    crate::utils::system::print_error(&err);
                    if md.show_critical_info()? {
                        open::that(output_file.path())?;
                        std::thread::sleep(Duration::from_millis(5000));
                    }
                }
                Err(err)
            }
        }
    }

    /// Dummy wrap call that never detaches for non mac platforms.
    #[cfg(not(target_os = "macos"))]
    pub fn wrap<T, F: FnOnce(&mut MayDetach) -> Result<T>>(task_name: &'a str, f: F) -> Result<T> {
        f(&mut MayDetach::new(task_name))
    }

    #[cfg(target_os = "macos")]
    fn show_critical_info(&self) -> Result<bool> {
        show_critical_info(
            &format!("{} failed", self.task_name),
            "The Sentry build step failed while running in the background. \
             You can ignore this error or view details to attempt to resolve \
             it. Ignoring it might cause your crashes not to be handled \
             properly.",
        )
    }

    #[cfg(target_os = "macos")]
    fn show_done(&self) -> Result<()> {
        if self.is_detached() {
            show_notification("Sentry", &format!("{} finished", self.task_name))?;
        }
        Ok(())
    }
}

/// Returns true if we were invoked from xcode
#[cfg(target_os = "macos")]
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
            if name == "Xcode" {
                return true;
            }
        }
        pid = parent;
    }

    false
}

/// Returns true if we were invoked from xcode
#[cfg(not(target_os = "macos"))]
pub fn launched_from_xcode() -> bool {
    false
}

/// Shows a dialog in xcode and blocks.  The dialog will have a title and a
/// message as well as the buttons "Show details" and "Ignore".  Returns
/// `true` if the `show details` button has been pressed.
#[cfg(target_os = "macos")]
pub fn show_critical_info(title: &str, message: &str) -> Result<bool> {
    use serde::Serialize;

    lazy_static! {
        static ref SCRIPT: osascript::JavaScript = osascript::JavaScript::new(
            "
            var App = Application('XCode');
            App.includeStandardAdditions = true;
            return App.displayAlert($params.title, {
                message: $params.message,
                as: \"critical\",
                buttons: [\"Show details\", \"Ignore\"]
            });
        "
        );
    }

    #[derive(Serialize)]
    struct AlertParams<'a> {
        title: &'a str,
        message: &'a str,
    }

    #[derive(Debug, Deserialize)]
    struct AlertResult {
        #[serde(rename = "buttonReturned")]
        button: String,
    }

    let rv: AlertResult = SCRIPT
        .execute_with_params(AlertParams { title, message })
        .context("Failed to display Xcode dialog")?;

    Ok(&rv.button != "Ignore")
}

/// Shows a notification in xcode
#[cfg(target_os = "macos")]
pub fn show_notification(title: &str, message: &str) -> Result<()> {
    use crate::config::Config;
    use serde::Serialize;

    lazy_static! {
        static ref SCRIPT: osascript::JavaScript = osascript::JavaScript::new(
            "
            var App = Application.currentApplication();
            App.includeStandardAdditions = true;
            App.displayNotification($params.message, {
                withTitle: $params.title
            });
        "
        );
    }

    let config = Config::current();
    if !config.show_notifications()? {
        return Ok(());
    }

    #[derive(Serialize)]
    struct NotificationParams<'a> {
        title: &'a str,
        message: &'a str,
    }

    SCRIPT
        .execute_with_params(NotificationParams { title, message })
        .context("Failed to display Xcode notification")?;

    Ok(())
}

#[test]
fn test_expansion() {
    let mut vars = HashMap::new();
    vars.insert("FOO_BAR".to_string(), "foo bar baz / blah".to_string());

    assert_eq!(
        expand_xcodevars("A$(FOO_BAR:rfc1034identifier)B", &vars),
        "Afoo-bar-baz-blahB"
    );
    assert_eq!(
        expand_xcodevars("A$(FOO_BAR:identifier)B", &vars),
        "Afoo_bar_baz_blahB"
    );
    assert_eq!(
        expand_xcodevars("A${FOO_BAR:identifier}B", &vars),
        "Afoo_bar_baz_blahB"
    );
}
