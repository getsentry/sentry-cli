use std::fs;
use std::env;
use std::fmt;
use std::path::Path;
use std::io::BufReader;
use std::borrow::Cow;

use plist::serde::deserialize;
use walkdir::WalkDir;

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
