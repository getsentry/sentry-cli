use std::fmt;
use std::str;
use std::path::Path;
use std::ffi::OsStr;
use std::collections::BTreeMap;

use serde::ser::{Serialize, SerializeStruct, Serializer};
use symbolic_common::{ByteView, ObjectKind};
use symbolic_debuginfo::{FatObject, Object, ObjectId, SymbolTable};
use symbolic_proguard::ProguardMappingView;

use errors::{Error, Result};

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone, Serialize)]
pub enum DifType {
    #[serde(rename = "dsym")] Dsym,
    #[serde(rename = "breakpad")] Breakpad,
    #[serde(rename = "proguard")] Proguard,
}

impl fmt::Display for DifType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DifType::Dsym => write!(f, "dsym"),
            &DifType::Breakpad => write!(f, "breakpad"),
            &DifType::Proguard => write!(f, "proguard"),
        }
    }
}

impl str::FromStr for DifType {
    type Err = Error;

    fn from_str(s: &str) -> Result<DifType> {
        match s {
            "dsym" => Ok(DifType::Dsym),
            "breakpad" => Ok(DifType::Breakpad),
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
                ObjectKind::Breakpad => return DifFile::from_object(fat),
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
            Some(DifType::Breakpad) => DifFile::open_object(path, ObjectKind::Breakpad),
            Some(DifType::Proguard) => DifFile::open_proguard(path),
            None => DifFile::try_open(path),
        }
    }

    pub fn ty(&self) -> DifType {
        match self {
            &DifFile::Object(ref fat) => match fat.kind() {
                ObjectKind::MachO => DifType::Dsym,
                ObjectKind::Breakpad => DifType::Breakpad,
                _ => unreachable!(),
            },
            &DifFile::Proguard(..) => DifType::Proguard,
        }
    }

    pub fn variants(&self) -> BTreeMap<ObjectId, Option<&'static str>> {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .filter_map(|object| object.id().map(|id| (id, Some(object.arch().name()))))
                .collect(),
            &DifFile::Proguard(ref pg) => vec![(pg.uuid().into(), None)].into_iter().collect(),
        }
    }

    pub fn ids(&self) -> Vec<ObjectId> {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .filter_map(|object| object.id())
                .collect(),
            &DifFile::Proguard(ref pg) => vec![pg.uuid().into()],
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            &DifFile::Object(ref fat) => fat.objects()
                .filter_map(|result| result.ok())
                .any(|object| object.debug_kind().is_some()),
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
        if self.has_hidden_symbols().unwrap_or(false) {
            Some("contains hidden symbols (needs BCSymbolMaps)")
        } else {
            None
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

/// A trait to help interfacing with debugging information.
pub trait DebuggingInformation {
    /// Checks whether this object contains hidden symbols generated during an
    /// iTunes Connect build. This only applies to MachO files.
    fn has_hidden_symbols(&self) -> Result<bool>;
}

impl DebuggingInformation for DifFile {
    fn has_hidden_symbols(&self) -> Result<bool> {
        match *self {
            DifFile::Object(ref fat) => fat.has_hidden_symbols(),
            _ => Ok(false),
        }
    }
}

impl<'a> DebuggingInformation for FatObject<'a> {
    fn has_hidden_symbols(&self) -> Result<bool> {
        if self.kind() != ObjectKind::MachO {
            return Ok(false);
        }

        for object in self.objects() {
            if object?.symbols()?.requires_symbolmap()? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl<'a> DebuggingInformation for Object<'a> {
    fn has_hidden_symbols(&self) -> Result<bool> {
        Ok(self.kind() == ObjectKind::MachO && self.symbols()?.requires_symbolmap()?)
    }
}
