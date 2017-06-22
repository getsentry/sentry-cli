use std::fmt;
use std::str;
use std::path::Path;
use std::ffi::OsStr;
use std::collections::BTreeMap;

use prelude::*;
use utils::MachoInfo;

use proguard;
use uuid::Uuid;
use serde::ser::{Serialize, Serializer, SerializeStruct};


#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone, Serialize)]
pub enum DifType {
    #[serde(rename="dsym")]
    Dsym,
    #[serde(rename="proguard")]
    Proguard,
}

impl fmt::Display for DifType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &DifType::Dsym => "dsym",
            &DifType::Proguard => "proguard",
        })
    }
}

impl str::FromStr for DifType {
    type Err = Error;

    fn from_str(s: &str) -> Result<DifType> {
        match s {
            "dsym" => Ok(DifType::Dsym),
            "proguard" => Ok(DifType::Proguard),
            _ => Err(Error::from("Invalid debug info file type")),
        }
    }
}

pub enum DifFile {
    Dsym(MachoInfo),
    Proguard(proguard::MappingView<'static>),
}

impl DifFile {
    pub fn open_path<P: AsRef<Path>>(p: P, ty: Option<DifType>) -> Result<DifFile> {
        let path = p.as_ref();
        Ok(match ty {
            Some(DifType::Dsym) => DifFile::Dsym(MachoInfo::open_path(&path)?),
            Some(DifType::Proguard) => DifFile::Proguard(proguard::MappingView::from_path(&path)?),
            None => {
                if let Ok(mi) = MachoInfo::open_path(&path) {
                    DifFile::Dsym(mi)
                } else {
                    match proguard::MappingView::from_path(&path) {
                        Ok(pg) => {
                            if path.extension() == Some(OsStr::new("txt")) ||
                               pg.has_line_info() {
                                DifFile::Proguard(pg)
                            } else {
                                fail!("invalid debug info file");
                            }
                        }
                        Err(err) => { return Err(err.into()) }
                    }
                }
            }
        })
    }

    pub fn ty(&self) -> DifType {
        match self {
            &DifFile::Dsym(..) => DifType::Dsym,
            &DifFile::Proguard(..) => DifType::Proguard,
        }
    }

    pub fn variants(&self) -> BTreeMap<Uuid, Option<&'static str>> {
        match self {
            &DifFile::Dsym(ref mi) => {
                mi.get_architectures()
                    .into_iter()
                    .map(|(key, value)| (key, Some(value)))
                    .collect()
            }
            &DifFile::Proguard(ref pg) => {
                vec![(pg.uuid(), None)].into_iter().collect()
            }
        }
    }

    pub fn uuids(&self) -> Vec<Uuid> {
        match self {
            &DifFile::Dsym(ref mi) => mi.get_uuids(),
            &DifFile::Proguard(ref pg) => vec![pg.uuid()],
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            &DifFile::Dsym(ref mi) => mi.has_debug_info(),
            &DifFile::Proguard(ref pg) => pg.has_line_info(),
        }
    }

    pub fn get_problem(&self) -> Option<&str> {
        if self.is_usable() {
            None
        } else {
            Some(match self {
                &DifFile::Dsym(..) => "missing DWARF debug info",
                &DifFile::Proguard(..) => "missing line information",
            })
        }
    }
}

impl Serialize for DifFile {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("DifFile", 4)?;
        state.serialize_field("type", &self.ty())?;
        state.serialize_field("variants", &self.variants())?;
        state.serialize_field("is_usable", &self.is_usable())?;
        state.serialize_field("problem", &self.get_problem())?;
        state.end()
    }
}
