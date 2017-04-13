//! Provides support for working with macho binaries.
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::fs::File;
use std::path::Path;
use std::collections::HashSet;

use prelude::*;

use memmap;
use uuid::Uuid;
use mach_object::{OFile, LoadCommand, MachCommand};


const FAT_MAGIC: &'static [u8; 4] = b"\xca\xfe\xba\xbe";
const MAGIC: &'static [u8; 4] = b"\xfe\xed\xfa\xce";
const MAGIC_CIGAM: &'static [u8; 4] = b"\xce\xfa\xed\xfe";
const MAGIC_64: &'static [u8; 4] = b"\xfe\xed\xfa\xcf";
const MAGIC_CIGAM64: &'static [u8; 4] = b"\xcf\xfa\xed\xfe";


/// this function can return an error if the file is smaller than the magic.
/// Use the `is_macho_file` instead which does not fail which is actually
/// much better for how this function is used within this library.
fn is_macho_file_as_result<R: Read>(mut rdr: R) -> Result<bool> {
    let mut magic: [u8; 4] = [0; 4];
    rdr.read_exact(&mut magic)?;
    Ok(match &magic {
        FAT_MAGIC | MAGIC | MAGIC_CIGAM | MAGIC_64 | MAGIC_CIGAM64 => true,
        _ => false,
    })
}

/// Simplified check for if a file is a macho binary.  Returns `true` if it
/// is or `false` if it's not (or the file does not exist etc.)
pub fn is_macho_file<R: Read>(rdr: R) -> bool {
    is_macho_file_as_result(rdr).unwrap_or(false)
}

fn get_macho_uuids_from_file(f: &File) -> Result<HashSet<Uuid>> {
    if let Ok(mmap) = memmap::Mmap::open(f, memmap::Protection::Read) {
        get_macho_uuids_from_slice(unsafe { mmap.as_slice() })
    } else {
        Ok(HashSet::new())
    }
}

fn get_macho_uuids_from_reader<R: Read>(mut rdr: R) -> Result<HashSet<Uuid>> {
    let mut contents: Vec<u8> = vec![];
    rdr.read_to_end(&mut contents)?;
    get_macho_uuids_from_slice(&contents[..])
}

pub fn is_matching_macho_reader<R: Read>(rdr: R, uuids: Option<&HashSet<Uuid>>) -> Result<bool> {
    if let Some(uuids) = uuids {
        let uuids_found = get_macho_uuids_from_reader(rdr)?;
        Ok(!uuids_found.is_empty() && uuids_found.is_subset(uuids))
    } else {
        Ok(is_macho_file(rdr))
    }
}

pub fn is_matching_macho_path<P: AsRef<Path>>(p: P, uuids: Option<&HashSet<Uuid>>) -> Result<bool> {
    let mut f = File::open(p)?;
    if let Some(uuids) = uuids {
        let uuids_found = get_macho_uuids_from_file(&f)?;
        Ok(!uuids_found.is_empty() && uuids_found.is_subset(uuids))
    } else {
        Ok(is_macho_file(f))
    }
}

fn get_macho_uuids_from_slice(slice: &[u8]) -> Result<HashSet<Uuid>> {
    let mut cursor = Cursor::new(slice);
    let mut uuids = HashSet::new();

    if !is_macho_file(&mut cursor) {
        return Ok(uuids);
    }
    cursor.seek(SeekFrom::Start(0))?;

    let ofile = OFile::parse(&mut cursor)?;

    match ofile {
        OFile::FatFile { ref files, .. } => {
            for &(_, ref file) in files {
                extract_uuids(&mut uuids, file);
            }
        }
        OFile::MachFile { .. } => {
            extract_uuids(&mut uuids, &ofile);
        }
        _ => {}
    }

    Ok(uuids)
}

fn extract_uuids<'a>(uuids: &'a mut HashSet<Uuid>, file: &'a OFile) {
    if let &OFile::MachFile { ref commands, .. } = file {
        for &MachCommand(ref load_cmd, _) in commands {
            match load_cmd {
                &LoadCommand::Uuid(uuid) => {
                    uuids.insert(uuid);
                },
                _ => {}
            }
        }
    }
}
