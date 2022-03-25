use std::fs;
use std::io::BufReader;
use std::path::Path;

use anyhow::Result;
use elementtree::{Element, QName};

pub struct CordovaConfig {
    root: Element,
}

impl CordovaConfig {
    pub fn load<P: AsRef<Path>>(p: P) -> Result<Option<CordovaConfig>> {
        let f = fs::File::open(p)?;
        let root = Element::from_reader(BufReader::new(f))?;
        if root.tag() != &QName::from("{http://www.w3.org/ns/widgets}widget") {
            Ok(None)
        } else {
            Ok(Some(CordovaConfig { root }))
        }
    }

    pub fn id(&self) -> &str {
        self.root.get_attr("id").unwrap_or("unknown")
    }

    pub fn version(&self) -> &str {
        self.root.get_attr("version").unwrap_or("0.0")
    }

    pub fn android_package(&self) -> &str {
        self.root
            .get_attr("android-packageName")
            .unwrap_or_else(|| self.id())
    }

    pub fn ios_bundle_identifier(&self) -> &str {
        self.root
            .get_attr("ios-CFBundleIdentifier")
            .unwrap_or_else(|| self.id())
    }

    pub fn ios_version(&self) -> &str {
        self.root
            .get_attr("ios-CFBundleVersion")
            .unwrap_or_else(|| self.version())
    }

    pub fn android_release_name(&self) -> String {
        format!("{}@{}", self.android_package(), self.version())
    }

    pub fn ios_release_name(&self) -> String {
        format!("{}@{}", self.ios_bundle_identifier(), self.ios_version())
    }
}
