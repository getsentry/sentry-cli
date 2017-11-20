//! Provides support for working with macho binaries.
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::fs::File;
use std::path::Path;
use std::collections::{HashSet, HashMap};
use std::rc::Rc;

use prelude::*;

use regex::Regex;
use memmap;
use uuid::Uuid;
use mach_object::{OFile, LoadCommand, MachCommand, Section, SymbolReader,
                  get_arch_name_from_types};


lazy_static! {
    static ref HIDDEN_SYMBOL_RE: Regex = Regex::new("__hidden#\\d+").unwrap();
}


const FAT_MAGIC: &'static [u8; 4] = b"\xca\xfe\xba\xbe";
const FAT_CIGAM: &'static [u8; 4] = b"\xbe\xba\xfe\xca";
const MAGIC: &'static [u8; 4] = b"\xfe\xed\xfa\xce";
const MAGIC_CIGAM: &'static [u8; 4] = b"\xce\xfa\xed\xfe";
const MAGIC_64: &'static [u8; 4] = b"\xfe\xed\xfa\xcf";
const MAGIC_CIGAM64: &'static [u8; 4] = b"\xcf\xfa\xed\xfe";

pub struct MachoInfo {
    uuids: HashMap<Uuid, &'static str>,
    has_dwarf_data: bool,
    has_hidden_symbols: bool,
}

impl MachoInfo {

    pub fn open_path(path: &Path) -> Result<MachoInfo> {
        let f = File::open(path)?;
        if let Ok(mmap) = memmap::Mmap::open(&f, memmap::Protection::Read) {
            MachoInfo::from_slice(unsafe { mmap.as_slice() })
        } else {
            Err(ErrorKind::NoMacho.into())
        }
    }

    pub fn from_reader<R: Read>(mut rdr: R) -> Result<MachoInfo> {
        let mut contents: Vec<u8> = vec![];
        rdr.read_to_end(&mut contents)?;
        MachoInfo::from_slice(&contents[..])
    }

    pub fn from_slice(slice: &[u8]) -> Result<MachoInfo> {
        fn find_dwarf_section<'a>(rv: &mut MachoInfo, sections: &[Rc<Section>]) {
            for sect in sections {
                if sect.segname == "__DWARF" {
                    rv.has_dwarf_data = true;
                }
            }
        }

        fn extract_info<'a>(rv: &mut MachoInfo, file: &'a OFile, cur: &'a mut Cursor<&'a [u8]>) {
            if let &OFile::MachFile { ref header, ref commands, .. } = file {
                for &MachCommand(ref load_cmd, _) in commands {
                    match load_cmd {
                        &LoadCommand::Uuid(uuid) => {
                            rv.uuids.insert(uuid, get_arch_name_from_types(
                                header.cputype, header.cpusubtype).unwrap_or("unknown"));
                        },
                        &LoadCommand::Segment { ref sections, .. } => {
                            find_dwarf_section(rv, &sections[..]);
                        }
                        &LoadCommand::Segment64 { ref sections, .. } => {
                            find_dwarf_section(rv, &sections[..]);
                        }
                        _ => {}
                    }
                }
                if !rv.has_hidden_symbols {
                    if let Some(iter) = file.symbols(cur) {
                        for symbol in iter {
                            if_chain! {
                                if !symbol.is_external();
                                if let Some(sym) = symbol.name();
                                if HIDDEN_SYMBOL_RE.is_match(sym);
                                then {
                                    rv.has_hidden_symbols = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut rv = MachoInfo {
            uuids: HashMap::new(),
            has_dwarf_data: false,
            has_hidden_symbols: false,
        };
        let mut cursor = Cursor::new(slice);

        if !is_macho_file(&mut cursor) {
            return Err(ErrorKind::NoMacho.into());
        }
        cursor.seek(SeekFrom::Start(0))?;

        let ofile = OFile::parse(&mut cursor)?;
        match ofile {
            OFile::FatFile { ref files, .. } => {
                for &(ref arch, ref file) in files {
                    let mut f_cur = Cursor::new(&slice[arch.offset as usize..]);
                    extract_info(&mut rv, file, &mut f_cur);
                }
            }
            OFile::MachFile { .. } => {
                let mut f_cur = Cursor::new(slice);
                extract_info(&mut rv, &ofile, &mut f_cur);
            }
            _ => {}
        }

        Ok(rv)
    }

    pub fn has_debug_info(&self) -> bool {
        self.has_dwarf_data && !self.uuids.is_empty()
    }

    pub fn has_hidden_symbols(&self) -> bool {
        self.has_hidden_symbols
    }

    pub fn matches_any(&self, uuids: &HashSet<Uuid>) -> bool {
        for uuid in uuids {
            if self.uuids.contains_key(uuid) {
                return true;
            }
        }
        false
    }

    pub fn get_uuids(&self) -> Vec<Uuid> {
        self.uuids.iter().map(|x| *x.0).collect()
    }

    pub fn get_architectures(&self) -> HashMap<Uuid, &'static str> {
        self.uuids.clone()
    }
}


/// this function can return an error if the file is smaller than the magic.
/// Use the `is_macho_file` instead which does not fail which is actually
/// much better for how this function is used within this library.
fn is_macho_file_as_result<R: Read>(mut rdr: R) -> Result<bool> {
    let mut magic: [u8; 4] = [0; 4];
    rdr.read_exact(&mut magic)?;
    Ok(match &magic {
        FAT_MAGIC | FAT_CIGAM | MAGIC | MAGIC_CIGAM | MAGIC_64 | MAGIC_CIGAM64 => true,
        _ => false,
    })
}

/// Simplified check for if a file is a macho binary.  Returns `true` if it
/// is or `false` if it's not (or the file does not exist etc.)
pub fn is_macho_file<R: Read>(rdr: R) -> bool {
    is_macho_file_as_result(rdr).unwrap_or(false)
}
