use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{format_err, Result};
use itertools::Itertools;
use uuid::Uuid;

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
