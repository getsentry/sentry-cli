use std::fs;
use std::env;
use std::fmt;
use std::path::Path;
use std::io::BufReader;
use std::borrow::Cow;

use plist::serde::deserialize;
use walkdir::WalkDir;
#[cfg(target_os="macos")]
use osascript;

use prelude::*;
use utils::expand_envvars;


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
