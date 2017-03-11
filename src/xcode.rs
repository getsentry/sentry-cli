use std::fs;
use std::path::Path;
use std::io::BufReader;

use plist::serde::deserialize;

use prelude::*;


#[derive(Deserialize, Debug)]
pub struct InfoPlist {
    #[serde(rename="CFBundleShortVersionString")]
    version: String,
    #[serde(rename="CFBundleVersion")]
    build: String,
}

impl InfoPlist {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<InfoPlist> {
        let f = fs::File::open(path.as_ref())?;
        let mut rdr = BufReader::new(f);
        Ok(deserialize(&mut rdr)?)
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
}
