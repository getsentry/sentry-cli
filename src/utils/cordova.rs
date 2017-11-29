use std::fs;
use std::path::Path;
use std::io::BufReader;

use elementtree::{Element, QName};

use prelude::*;


pub struct CordovaConfig {
    id: String,
    version: String,
}

impl CordovaConfig {
    pub fn load<P: AsRef<Path>>(p: P) -> Result<Option<CordovaConfig>> {
        let mut f = fs::File::open(p)?;
        let root = Element::from_reader(BufReader::new(f))?;
        if root.tag() != &QName::from("{http://www.w3.org/ns/widgets}widget") {
            return Ok(None);
        }
        Ok(Some(CordovaConfig {
            id: match root.get_attr("id") {
                Some(value) => value.to_string(),
                None => { return Ok(None); }
            },
            version: match root.get_attr("version") {
                Some(value) => value.to_string(),
                None => { return Ok(None); }
            },
        }))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}
