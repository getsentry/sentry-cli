use std::fmt;
use std::path::Path;
use std::str;

use anyhow::{bail, Error, Result};
use proguard::ProguardMapping;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use symbolic::common::{ByteView, CodeId, DebugId, SelfCell};
use symbolic::debuginfo::{Archive, FileFormat, Object, ObjectKind};

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
    PortablePdb,
    Wasm,
}

impl DifType {
    pub fn name(self) -> &'static str {
        match self {
            DifType::Dsym => "dsym",
            DifType::Elf => "elf",
            DifType::Pe => "pe",
            DifType::Pdb => "pdb",
            DifType::PortablePdb => "portablepdb",
            DifType::SourceBundle => "sourcebundle",
            DifType::Breakpad => "breakpad",
            DifType::Proguard => "proguard",
            DifType::Wasm => "wasm",
        }
    }

    pub fn all() -> &'static [DifType] {
        &[
            DifType::Dsym,
            DifType::Elf,
            DifType::Pe,
            DifType::Pdb,
            DifType::PortablePdb,
            DifType::SourceBundle,
            DifType::Breakpad,
            DifType::Proguard,
            DifType::Wasm,
        ]
    }

    pub fn all_names() -> &'static [&'static str] {
        &[
            "dsym",
            "elf",
            "pe",
            "pdb",
            "portablepdb",
            "sourcebundle",
            "breakpad",
            "proguard",
            "wasm",
        ]
    }
}

impl fmt::Display for DifType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl str::FromStr for DifType {
    type Err = Error;

    fn from_str(s: &str) -> Result<DifType> {
        match s {
            "dsym" => Ok(DifType::Dsym),
            "elf" => Ok(DifType::Elf),
            "pe" => Ok(DifType::Pe),
            "pdb" => Ok(DifType::Pdb),
            "portablepdb" => Ok(DifType::PortablePdb),
            "sourcebundle" => Ok(DifType::SourceBundle),
            "breakpad" => Ok(DifType::Breakpad),
            "proguard" => Ok(DifType::Proguard),
            "wasm" => Ok(DifType::Wasm),
            _ => bail!("Invalid debug info file type"),
        }
    }
}

/// Declares which features an object may have to be uploaded.
#[derive(Clone, Copy, Debug)]
pub struct ObjectDifFeatures {
    /// Includes object files with debug information.
    pub debug: bool,
    /// Includes object files with a symbol table.
    pub symtab: bool,
    /// Includes object files with stack unwind information.
    pub unwind: bool,
    /// Includes source information.
    pub sources: bool,
}

impl ObjectDifFeatures {
    pub fn all() -> Self {
        ObjectDifFeatures {
            debug: true,
            symtab: true,
            unwind: true,
            sources: true,
        }
    }

    pub fn none() -> Self {
        ObjectDifFeatures {
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

impl Default for ObjectDifFeatures {
    fn default() -> Self {
        Self::all()
    }
}

impl fmt::Display for ObjectDifFeatures {
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

pub struct SelfProguard<'a>(ProguardMapping<'a>);

impl<'a> std::ops::Deref for SelfProguard<'a> {
    type Target = ProguardMapping<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'slf> symbolic::common::AsSelf<'slf> for SelfProguard<'_> {
    type Ref = SelfProguard<'slf>;

    fn as_self(&'slf self) -> &Self::Ref {
        self
    }
}

pub enum DifFile<'a> {
    Archive(SelfCell<ByteView<'a>, Archive<'a>>),
    Proguard(SelfCell<ByteView<'a>, SelfProguard<'a>>),
}

impl DifFile<'static> {
    fn open_proguard<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = ByteView::open(path).map_err(Error::new)?;
        let pg = SelfCell::new(data, |d| SelfProguard(ProguardMapping::new(unsafe { &*d })));

        if pg.get().is_valid() {
            Ok(DifFile::Proguard(pg))
        } else {
            bail!("Expected a proguard file")
        }
    }

    fn open_object<P: AsRef<Path>>(path: P, format: FileFormat) -> Result<Self> {
        let data = ByteView::open(path).map_err(Error::new)?;
        let archive = SelfCell::try_new(data, |d| Archive::parse(unsafe { &*d }))?;

        if archive.get().file_format() != format {
            bail!("Unexpected file format");
        }

        DifFile::from_archive(archive)
    }

    fn try_open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Try to open the file and map it into memory first. This will
        // return an error if the file does not exist.
        let data = ByteView::open(&path).map_err(Error::new)?;

        // First try to open a (fat) object file. We only support a couple of
        // sub types, so for unsupported files we throw an error.
        if let Ok(archive) = SelfCell::try_new(data, |d| Archive::parse(unsafe { &*d })) {
            match archive.get().file_format() {
                FileFormat::MachO
                | FileFormat::Elf
                | FileFormat::Pe
                | FileFormat::Pdb
                | FileFormat::PortablePdb
                | FileFormat::Breakpad
                | FileFormat::Wasm
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

    pub fn open_path<P: AsRef<Path>>(path: P, ty: Option<DifType>) -> Result<Self> {
        match ty {
            Some(DifType::Dsym) => DifFile::open_object(path, FileFormat::MachO),
            Some(DifType::Elf) => DifFile::open_object(path, FileFormat::Elf),
            Some(DifType::Pe) => DifFile::open_object(path, FileFormat::Pe),
            Some(DifType::Pdb) => DifFile::open_object(path, FileFormat::Pdb),
            Some(DifType::PortablePdb) => DifFile::open_object(path, FileFormat::PortablePdb),
            Some(DifType::SourceBundle) => DifFile::open_object(path, FileFormat::SourceBundle),
            Some(DifType::Wasm) => DifFile::open_object(path, FileFormat::Wasm),
            Some(DifType::Breakpad) => DifFile::open_object(path, FileFormat::Breakpad),
            Some(DifType::Proguard) => DifFile::open_proguard(path),
            None => DifFile::try_open(path),
        }
    }
}

#[derive(Serialize)]
pub struct DifVariant {
    pub debug_id: DebugId,
    pub code_id: Option<CodeId>,
    pub arch: Option<String>,
}

impl<'a> DifFile<'a> {
    fn from_archive(archive: SelfCell<ByteView<'a>, Archive<'a>>) -> Result<Self> {
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
                FileFormat::PortablePdb => DifType::PortablePdb,
                FileFormat::Pe => DifType::Pe,
                FileFormat::Wasm => DifType::Wasm,
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

    pub fn variants(&self) -> Vec<DifVariant> {
        match self {
            DifFile::Archive(archive) => archive
                .get()
                .objects()
                .filter_map(Result::ok)
                .map(|object| DifVariant {
                    debug_id: object.debug_id(),
                    arch: Some(object.arch().name().to_string()),
                    code_id: object.code_id(),
                })
                .collect(),
            DifFile::Proguard(pg) => vec![DifVariant {
                debug_id: pg.get().uuid().into(),
                arch: None,
                code_id: None,
            }],
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
            DifFile::Proguard(pg) => vec![pg.get().uuid().into()],
        }
    }

    pub fn features(&self) -> ObjectDifFeatures {
        match self {
            DifFile::Archive(archive) => {
                let mut features = ObjectDifFeatures::none();

                let mut add_object_features = |object: &Object| {
                    features.symtab = features.symtab || object.has_symbols();
                    features.debug = features.debug || object.has_debug_info();
                    features.unwind = features.unwind || object.has_unwind_info();
                    features.sources = features.sources || object.has_sources();
                };

                for object in archive.get().objects().filter_map(Result::ok) {
                    add_object_features(&object);

                    // Combine features with an embedded Portable PDB, if any to show up correctly in `dif check` cmd.
                    // Note: this is intentionally different than `DifUpload.valid_features()` because we don't want to
                    // upload the PE file separately, unless it has features we need. The PPDB is extracted instead.
                    if let Ok(Some(Object::Pe(pe))) = archive.get().object_by_index(0) {
                        if let Ok(Some(ppdb_data)) = pe.embedded_ppdb() {
                            let mut buf = Vec::new();
                            if ppdb_data.decompress_to(&mut buf).is_ok() {
                                if let Ok(ppdb) = Object::parse(&buf) {
                                    add_object_features(&ppdb);
                                }
                            }
                        }
                    }
                }
                features
            }
            DifFile::Proguard(..) => ObjectDifFeatures::none(),
        }
    }

    pub fn is_usable(&self) -> bool {
        match self {
            DifFile::Archive(_) => self.has_ids() && self.features().has_some(),
            DifFile::Proguard(pg) => pg.get().has_line_info(),
        }
    }

    pub fn get_problem(&self) -> Option<&'static str> {
        if self.is_usable() {
            None
        } else {
            Some(match self {
                DifFile::Archive(..) => {
                    if !self.has_ids() {
                        "missing debug identifier, likely stripped"
                    } else {
                        "missing debug or unwind information"
                    }
                }
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

    fn has_ids(&self) -> bool {
        self.ids().iter().any(|id| !id.is_nil())
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
    fn has_hidden_symbols(&self) -> Result<bool>;
}

impl DebuggingInformation for DifFile<'_> {
    fn has_hidden_symbols(&self) -> Result<bool> {
        match self {
            DifFile::Archive(archive) => archive.get().has_hidden_symbols(),
            _ => Ok(false),
        }
    }
}

impl DebuggingInformation for Archive<'_> {
    fn has_hidden_symbols(&self) -> Result<bool> {
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
