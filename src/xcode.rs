use std::fs;
use std::env;
use std::fmt;
use std::path::Path;
use std::io::BufReader;
use std::borrow::Cow;
use std::thread;
use std::time::Duration;

use plist::serde::deserialize;
use walkdir::WalkDir;
#[cfg(target_os="macos")]
use osascript;
#[cfg(target_os="macos")]
use unix_daemonize::{daemonize_redirect, ChdirMode};
use open;

use prelude::*;
use utils::{TempFile, expand_envvars, print_error};


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

impl fmt::Display for InfoPlist {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name(), &self.version)
    }
}

impl InfoPlist {

    pub fn discover_from_path<P: AsRef<Path>>(path: P) -> Result<Option<InfoPlist>> {
        let fpl_fn = Some("info.plist".to_string());
        for dent_res in WalkDir::new(path.as_ref()) {
            let dent = dent_res?;
            if dent.file_name().to_str().map(|x| x.to_lowercase()) == fpl_fn {
                let md = dent.metadata()?;
                if md.is_file() {
                    return Ok(Some(InfoPlist::from_path(dent.path())?));
                }
            }
        }
        Ok(None)
    }

    pub fn discover_from_env() -> Result<Option<InfoPlist>> {
        if let Ok(path) = env::var("INFOPLIST_FILE") {
            Ok(Some(InfoPlist::from_path(path)?))
        } else {
            Ok(None)
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<InfoPlist> {
        let f = fs::File::open(path.as_ref()).chain_err(||
            Error::from("Could not open Info.plist file"))?;
        let mut rdr = BufReader::new(f);
        Ok(deserialize(&mut rdr).chain_err(||
            Error::from("Could not parse Info.plist file"))?)
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn build(&self) -> &str {
        &self.build
    }

    pub fn release_name(&self) -> String {
        format!("{} ({})", self.version, self.build)
    }

    pub fn name<'a>(&'a self) -> Cow<'a, str> {
        expand_envvars(&self.name)
    }

    pub fn bundle_id<'a>(&'a self) -> Cow<'a, str> {
        expand_envvars(&self.bundle_id)
    }
}

/// Helper struct that allows the current execution to detach from
/// the xcode console and continue in the background.  This becomes
/// a dummy shim for non xcode runs or platforms.
pub struct MayDetach<'a> {
    output_file: Option<TempFile>,
    task_name: &'a str,
}

impl<'a> MayDetach<'a> {
    fn new(task_name: &'a str) -> MayDetach<'a> {
        MayDetach {
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
        show_notification("Sentry", &format!("{} starting", self.task_name))?;
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
    pub fn wrap<T, F: FnOnce(&mut MayDetach) -> Result<T>>(task_name: &'a str, f: F) -> Result<T> {
        let mut md = MayDetach::new(task_name);
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
    pub fn wrap<T, F: FnOnce(&mut MayDetach) -> Result<T>>(task_name: &'a str, f: F) -> Result<T> {
        f(&mut MayDetach::new(task_name))
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
            show_notification("Sentry", &format!("{} finished", self.task_name))?;
        }
        Ok(())
    }
}

/// Returns true if we were invoked from xcode
#[cfg(target_os="macos")]
pub fn launched_from_xcode() -> bool {
    env::var("XCODE_VERSION_ACTUAL").is_ok() && env::var("TERM").is_err()
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
pub fn show_notification(title: &str, msg: &str) -> Result<()> {
    lazy_static! {
        static ref SCRIPT: osascript::JavaScript = osascript::JavaScript::new("
            var App = Application.currentApplication();
            App.includeStandardAdditions = true;
            App.displayNotification($params.message, {
                withTitle: $params.title
            });
        ");
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
