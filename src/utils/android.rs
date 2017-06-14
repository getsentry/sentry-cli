use std::fs;
use std::fmt;
use std::path::{Path, PathBuf};

use prelude::*;

use uuid::Uuid;
use elementtree::Element;
use itertools::Itertools;

pub struct AndroidManifest {
    path: PathBuf,
    root: Element,
}

const ANDROID_NS: &'static str = "http://schemas.android.com/apk/res/android";
const UUIDS_TAG: &'static str = "io.sentry.ProguardUuids";

impl AndroidManifest {

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<AndroidManifest> {
        let f = fs::File::open(path.as_ref())?;
        let root = Element::from_reader(f)?;
        Ok(AndroidManifest {
            path: path.as_ref().to_path_buf(),
            root: root,
        })
    }

    /// Returns the package ID
    pub fn package(&self) -> &str {
        self.root.get_attr("package").unwrap_or("unknown")
    }

    /// Returns a name
    pub fn name(&self) -> String {
        self.root.get_attr("package")
            .unwrap_or("unknown")
            .rsplit(".")
            .next()
            .unwrap()
            .chars()
            .enumerate()
            .map(|(idx, c)| {
                if idx == 0 {
                    c.to_uppercase().to_string()
                } else {
                    c.to_lowercase().to_string()
                }
            })
            .collect()
    }

    /// Returns the internal version code for this manifest
    pub fn version_code(&self) -> &str {
        self.root.get_attr((ANDROID_NS, "versionCode")).unwrap_or("0")
    }

    /// Returns the human readable version number of the manifest
    pub fn version_name(&self) -> &str {
        self.root.get_attr((ANDROID_NS, "versionName")).unwrap_or("0.0")
    }

    /// Returns the proguard uuids mentioned in the manifest
    pub fn proguard_uuids(&self) -> Vec<Uuid> {
        let mut rv = vec![];
        if let Some(app) = self.root.find("application") {
            for md in app.find_all("meta-data") {
                if md.get_attr((ANDROID_NS, "name")) == Some(UUIDS_TAG) {
                    let val = md.get_attr((ANDROID_NS, "value")).unwrap_or("");
                    for key in val.split('|') {
                        if let Ok(uuid) = key.parse() {
                            rv.push(uuid);
                        }
                    }
                }
            }
        }
        rv
    }

    /// Sets new values for the proguard uuids in the manifest
    pub fn set_proguard_uuids(&mut self, uuids: &[Uuid]) {
        let s = uuids.iter()
            .map(|x| x.to_string())
            .join("|");

        if let Some(mut app) = self.root.find_mut("application") {
            for mut md in app.find_all_mut("meta-data") {
                if md.get_attr((ANDROID_NS, "name")) == Some(UUIDS_TAG) {
                    md.set_attr((ANDROID_NS, "value"), s);
                    return;
                }
            }

            app.append_new_child("meta-data")
                .set_attr((ANDROID_NS, "name"), UUIDS_TAG)
                .set_attr((ANDROID_NS, "value"), s);
        }
    }

    /// Write back the file.
    pub fn save(&self) -> Result<()> {
        let mut f = fs::File::create(&self.path)?;
        self.root.to_writer(&mut f)?;
        Ok(())
    }
}

impl fmt::Debug for AndroidManifest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AndroidManifest")
            .field("package", &self.package())
            .field("version_code", &self.version_code())
            .field("version_name", &self.version_name())
            .field("proguard_uuids", &self.proguard_uuids())
            .finish()
    }
}
