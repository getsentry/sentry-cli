use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{format_err, Result};
use elementtree::Element;
use itertools::Itertools;
use uuid::Uuid;

pub struct AndroidManifest {
    root: Element,
}

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

impl AndroidManifest {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<AndroidManifest> {
        let f = fs::File::open(path.as_ref())?;
        let root = Element::from_reader(f)?;
        Ok(AndroidManifest { root })
    }

    /// Returns the package ID
    pub fn package(&self) -> &str {
        self.root.get_attr("package").unwrap_or("unknown")
    }

    /// Returns a name
    pub fn name(&self) -> String {
        // fallback name is the package reformatted
        self.root
            .get_attr("package")
            .unwrap_or("unknown")
            .rsplit('.')
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
        self.root
            .get_attr((ANDROID_NS, "versionCode"))
            .unwrap_or("0")
    }

    /// Returns the human readable version number of the manifest
    pub fn version_name(&self) -> &str {
        self.root
            .get_attr((ANDROID_NS, "versionName"))
            .unwrap_or("0.0")
    }
}

impl fmt::Debug for AndroidManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AndroidManifest")
            .field("package", &self.package())
            .field("version_code", &self.version_code())
            .field("version_name", &self.version_name())
            .finish()
    }
}

pub fn dump_proguard_uuids_as_properties<P: AsRef<Path>>(p: P, uuids: &[Uuid]) -> Result<()> {
    let mut props = match fs::File::open(p.as_ref()) {
        Ok(f) => java_properties::read(f).unwrap_or_else(|_| HashMap::new()),
        Err(err) => {
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err.into());
            } else {
                HashMap::new()
            }
        }
    };

    props.insert(
        "io.sentry.ProguardUuids".to_string(),
        uuids.iter().map(Uuid::to_string).join("|"),
    );

    if let Some(ref parent) = p.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(p.as_ref())?;
    java_properties::write(&mut f, &props)
        .map_err(|_| format_err!("Could not persist proguard UUID in properties file"))?;
    Ok(())
}
