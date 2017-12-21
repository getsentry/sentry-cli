use std::fmt;
use std::str;
use std::path::Path;
use std::ffi::OsStr;
use std::collections::BTreeMap;

use uuid::Uuid;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use symbolic_common::{ByteView, ObjectKind};
use symbolic_debuginfo::{DwarfData, FatObject, SymbolTable};
use symbolic_proguard::ProguardMappingView;

use prelude::*;

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone, Serialize)]
pub enum DifType {
    #[serde(rename = "dsym")] Dsym,
    #[serde(rename = "proguard")] Proguard,
}

impl fmt::Display for DifType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DifType::Dsym => write!(f, "dsym"),
            &DifType::Proguard => write!(f, "proguard"),
        }
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
    Object(FatObject<'static>),
    Proguard(ProguardMappingView<'static>),
}

impl DifFile {
    fn from_object(fat: FatObject<'static>) -> Result<DifFile> {
        if fat.object_count() < 1 {
            return Err(Error::from("Object file is empty"));
        }

        Ok(DifFile::Object(fat))
    }

    fn open_proguard<P: AsRef<Path>>(path: P) -> Result<DifFile> {

        let data = ByteView::from_path(&path)?;
        let pg = ProguardMappingView::parse(data)?;

        if path.as_ref().extension() == Some(OsStr::new("txt")) || pg.has_line_info() {
            Ok(DifFile::Proguard(pg))
        } else {
            Err(Error::from("Expected a proguard file"))
        }
    }

    fn open_object<P: AsRef<Path>>(path: P, kind: ObjectKind) -> Result<DifFile> {
        let data = ByteView::from_path(path)?;
        let fat = FatObject::parse(data)?;

        if fat.kind() != kind {
            return Err(Error::from("Unexpected file format"));
        }

        DifFile::from_object(fat)
    }

    fn try_open<P: AsRef<Path>>(path: P) -> Result<DifFile> {
        // Try to open the file and map it into memory first. This will
        // return an error if the file does not exist.
        let data = ByteView::from_path(&path)?;

        // First try to open a (fat) object file. We only support a couple of
        // sub types, so for unsupported files we throw an error.
        if let Ok(fat) = FatObject::parse(data) {
            match fat.kind() {
                ObjectKind::MachO => return DifFile::from_object(fat),
                _ => return Err(Error::from("Unsupported object file")),
            }
        }

        // Try opening as a proguard text file. This should be the last option
        // to try, as there is no reliable way to determine proguard files.
        if let Ok(dif) = DifFile::open_proguard(&path) {
            return Ok(dif);
        }

        // None of the above worked, so throw a generic error
        return Err(Error::from("Unsupported file"));
    }

    pub fn open_path<P: AsRef<Path>>(path: P, ty: Option<DifType>) -> Result<DifFile> {
        match ty {
            Some(DifType::Dsym) => DifFile::open_object(path, ObjectKind::MachO),
            Some(DifType::Proguard) => DifFile::open_proguard(path),
            None => DifFile::try_open(path),
        }
    }

    pub fn ty(&self) -> DifType {
        match self {
            &DifFile::Object(ref fat) => match fat.kind() {
                ObjectKind::MachO => DifType::Dsym,
                _ => unreachable!(),
            },
            &DifFile::Proguard(..) => DifType::Proguard,
        }
    }

    pub fn variants(&self) -> BTreeMap<Uuid, Option<&'static str>> {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .filter_map(|object| object.uuid().map(|uuid| (uuid, Some(object.arch().name()))))
                .collect(),
            &DifFile::Proguard(ref pg) => vec![(pg.uuid(), None)].into_iter().collect(),
        }
    }

    pub fn uuids(&self) -> Vec<Uuid> {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .filter_map(|object| object.uuid())
                .collect(),
            &DifFile::Proguard(ref pg) => vec![pg.uuid()],
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .any(|object| object.has_dwarf_data()),
            &DifFile::Proguard(ref pg) => pg.has_line_info(),
        }
    }

    pub fn get_problem(&self) -> Option<&str> {
        if self.is_usable() {
            None
        } else {
            Some(match self {
                &DifFile::Object(..) => "missing DWARF debug info",
                &DifFile::Proguard(..) => "missing line information",
            })
        }
    }

    pub fn get_note(&self) -> Option<&str> {
        match self {
            &DifFile::Object(ref fat) => {
                if has_hidden_symbols(fat).unwrap_or(false) {
                    Some("contains hidden symbols (needs BCSymbolMaps)")
                } else {
                    None
                }
            }
            &DifFile::Proguard(..) => None,
        }
    }
}

impl Serialize for DifFile {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 5 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("DifFile", 5)?;
        state.serialize_field("type", &self.ty())?;
        state.serialize_field("variants", &self.variants())?;
        state.serialize_field("is_usable", &self.is_usable())?;
        state.serialize_field("problem", &self.get_problem())?;
        state.serialize_field("note", &self.get_note())?;
        state.end()
    }
}

pub fn has_hidden_symbols(fat: &FatObject) -> Result<bool> {
    for object in fat.objects() {
        if object?.symbols()?.requires_symbolmap()? {
            return Ok(true);
        }
    }

    Ok(false)
}
