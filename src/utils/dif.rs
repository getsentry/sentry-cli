use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;
use std::str;

use failure::{bail, Error, SyncFailure};
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use symbolic::common::{ByteView, DebugId, SelfCell};
use symbolic::debuginfo::{Archive, FileFormat, Object, ObjectKind};
use symbolic::proguard::ProguardMappingView;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DifType {
    Dsym,
    Elf,
    Breakpad,
    Proguard,
    SourceBundle,
    Pe,
    Pdb,
}

impl DifType {
    pub fn name(self) -> &'static str {
        match self {
            DifType::Dsym => "dsym",
            DifType::Elf => "elf",
            DifType::Pe => "pe",
            DifType::Pdb => "pdb",
            DifType::SourceBundle => "sourcebundle",
            DifType::Breakpad => "breakpad",
            DifType::Proguard => "proguard",
        }
    }
}

impl fmt::Display for DifType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl str::FromStr for DifType {
    type Err = Error;

    fn from_str(s: &str) -> Result<DifType, Error> {
        match s {
            "dsym" => Ok(DifType::Dsym),
            "elf" => Ok(DifType::Elf),
            "pe" => Ok(DifType::Pe),
            "pdb" => Ok(DifType::Pdb),
            "sourcebundle" => Ok(DifType::SourceBundle),
            "breakpad" => Ok(DifType::Breakpad),
            "proguard" => Ok(DifType::Proguard),
            _ => bail!("Invalid debug info file type"),
        }
    }
}

/// Declares which features an object may have to be uploaded.
#[derive(Clone, Copy, Debug)]
pub struct DifFeatures {
    /// Includes object files with debug information.
    pub debug: bool,
    /// Includes object files with a symbol table.
    pub symtab: bool,
    /// Includes object files with stack unwind information.
    pub unwind: bool,
    /// Includes source information.
    pub sources: bool,
}

impl DifFeatures {
    pub fn all() -> Self {
        DifFeatures {
            debug: true,
            symtab: true,
            unwind: true,
            sources: true,
        }
    }

    pub fn none() -> Self {
        DifFeatures {
            debug: false,
            symtab: false,
            unwind: false,
            sources: false,
        }
    }

    fn has_some(self) -> bool {
        self.debug || self.symtab || self.unwind || self.sources
    }
}

impl Default for DifFeatures {
    fn default() -> Self {
        Self::all()
    }
}

impl fmt::Display for DifFeatures {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut written = false;

        macro_rules! append {
            ($condition:expr, $feature:literal) => {
                if $condition {
                    if written {
                        write!(f, ", ")?;
                    }
                    write!(f, $feature)?;
                    written = true;
                }
            };
        }

        append!(self.symtab, "symtab");
        append!(self.debug, "debug");
        append!(self.unwind, "unwind");
        append!(self.sources, "sources");

        if !written {
            write!(f, "none")?;
        }

        Ok(())
    }
}

pub enum DifFile<'a> {
    Archive(SelfCell<ByteView<'a>, Archive<'a>>),
    Proguard(ProguardMappingView<'a>),
}

impl DifFile<'static> {
    fn open_proguard<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let data = ByteView::open(&path).map_err(SyncFailure::new)?;
        let pg = ProguardMappingView::parse(data).map_err(SyncFailure::new)?;

        if path.as_ref().extension() == Some(OsStr::new("txt")) || pg.has_line_info() {
            Ok(DifFile::Proguard(pg))
        } else {
            bail!("Expected a proguard file")
        }
    }

    fn open_object<P: AsRef<Path>>(path: P, format: FileFormat) -> Result<Self, Error> {
        let data = ByteView::open(path).map_err(SyncFailure::new)?;
        let archive = SelfCell::try_new(data, |d| Archive::parse(unsafe { &*d }))?;

        if archive.get().file_format() != format {
            bail!("Unexpected file format");
        }

        DifFile::from_archive(archive)
    }

    fn try_open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        // Try to open the file and map it into memory first. This will
        // return an error if the file does not exist.
        let data = ByteView::open(&path).map_err(SyncFailure::new)?;

        // First try to open a (fat) object file. We only support a couple of
        // sub types, so for unsupported files we throw an error.
        if let Ok(archive) = SelfCell::try_new(data, |d| Archive::parse(unsafe { &*d })) {
            match archive.get().file_format() {
                FileFormat::MachO
                | FileFormat::Elf
                | FileFormat::Pe
                | FileFormat::Pdb
                | FileFormat::Breakpad
                | FileFormat::SourceBundle => return DifFile::from_archive(archive),
                FileFormat::Unknown => (), // fallthrough
            }
        }

        // Try opening as a proguard text file. This should be the last option
        // to try, as there is no reliable way to determine proguard files.
        if let Ok(dif) = DifFile::open_proguard(&path) {
            return Ok(dif);
        }

        // None of the above worked, so throw a generic error
        bail!("Unsupported file");
    }

    pub fn open_path<P: AsRef<Path>>(path: P, ty: Option<DifType>) -> Result<Self, Error> {
        match ty {
            Some(DifType::Dsym) => DifFile::open_object(path, FileFormat::MachO),
            Some(DifType::Elf) => DifFile::open_object(path, FileFormat::Elf),
            Some(DifType::Pe) => DifFile::open_object(path, FileFormat::Pe),
            Some(DifType::Pdb) => DifFile::open_object(path, FileFormat::Pdb),
            Some(DifType::SourceBundle) => DifFile::open_object(path, FileFormat::SourceBundle),
            Some(DifType::Breakpad) => DifFile::open_object(path, FileFormat::Breakpad),
            Some(DifType::Proguard) => DifFile::open_proguard(path),
            None => DifFile::try_open(path),
        }
    }
}

impl<'a> DifFile<'a> {
    fn from_archive(archive: SelfCell<ByteView<'a>, Archive<'a>>) -> Result<Self, Error> {
        if archive.get().object_count() < 1 {
            bail!("Object file is empty");
        }

        Ok(DifFile::Archive(archive))
    }

    pub fn ty(&self) -> DifType {
        match self {
            DifFile::Archive(archive) => match archive.get().file_format() {
                FileFormat::MachO => DifType::Dsym,
                FileFormat::Breakpad => DifType::Breakpad,
                FileFormat::Elf => DifType::Elf,
                FileFormat::Pdb => DifType::Pdb,
                FileFormat::Pe => DifType::Pe,
                FileFormat::SourceBundle => DifType::SourceBundle,
                FileFormat::Unknown => unreachable!(),
            },
            DifFile::Proguard(..) => DifType::Proguard,
        }
    }

    pub fn kind(&self) -> Option<ObjectKind> {
        match self {
            DifFile::Archive(archive) => match archive.get().object_by_index(0) {
                Ok(Some(object)) => Some(object.kind()),
                _ => None,
            },
            DifFile::Proguard(..) => None,
        }
    }

    pub fn variants(&self) -> BTreeMap<DebugId, Option<&'static str>> {
        match self {
            DifFile::Archive(archive) => archive
                .get()
                .objects()
                .filter_map(Result::ok)
                .map(|object| (object.debug_id(), Some(object.arch().name())))
                .collect(),
            DifFile::Proguard(pg) => vec![(pg.uuid().into(), None)].into_iter().collect(),
        }
    }

    pub fn ids(&self) -> Vec<DebugId> {
        match self {
            DifFile::Archive(archive) => archive
                .get()
                .objects()
                .filter_map(Result::ok)
                .map(|object| object.debug_id())
                .collect(),
            DifFile::Proguard(pg) => vec![pg.uuid().into()],
        }
    }

    pub fn features(&self) -> DifFeatures {
        match self {
            DifFile::Archive(archive) => {
                let mut features = DifFeatures::none();
                for object in archive.get().objects().filter_map(Result::ok) {
                    features.symtab = features.symtab || object.has_symbols();
                    features.debug = features.debug || object.has_debug_info();
                    features.unwind = features.unwind || object.has_unwind_info();
                    features.sources = features.sources || object.has_sources();
                }
                features
            }
            DifFile::Proguard(..) => DifFeatures::none(),
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            DifFile::Archive(_) => self.features().has_some(),
            DifFile::Proguard(pg) => pg.has_line_info(),
        }
    }

    pub fn get_problem(&self) -> Option<&'static str> {
        if self.is_usable() {
            None
        } else {
            Some(match self {
                DifFile::Archive(..) => "missing debug or unwind information",
                DifFile::Proguard(..) => "missing line information",
            })
        }
    }

    pub fn get_note(&self) -> Option<&'static str> {
        if self.has_hidden_symbols().unwrap_or(false) {
            Some("contains hidden symbols (needs BCSymbolMaps)")
        } else {
            None
        }
    }
}

impl Serialize for DifFile<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DifFile", 6)?;
        state.serialize_field("type", &self.ty())?;
        state.serialize_field("variants", &self.variants())?;
        state.serialize_field("features", &self.features().to_string())?;
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
    fn has_hidden_symbols(&self) -> Result<bool, Error>;
}

impl DebuggingInformation for DifFile<'_> {
    fn has_hidden_symbols(&self) -> Result<bool, Error> {
        match self {
            DifFile::Archive(archive) => archive.get().has_hidden_symbols(),
            _ => Ok(false),
        }
    }
}

impl DebuggingInformation for Archive<'_> {
    fn has_hidden_symbols(&self) -> Result<bool, Error> {
        // Hidden symbols can only ever occur in Apple's dSYM
        if self.file_format() != FileFormat::MachO {
            return Ok(false);
        }

        for object in self.objects() {
            if let Object::MachO(macho) = object? {
                if macho.requires_symbolmap() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
