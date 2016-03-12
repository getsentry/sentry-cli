use std::path::Path;
use std::fs::File;
use std::io::Read;

use super::CliResult;


const FAT_MAGIC : &'static [u8; 4] = b"\xca\xfe\xba\xbe";
const MAGIC : &'static [u8; 4] = b"\xfe\xed\xfa\xce";
const MAGIC_CIGAM : &'static [u8; 4] = b"\xce\xfa\xed\xfe";
const MAGIC_64 : &'static [u8; 4] = b"\xfe\xed\xfa\xcf";
const MAGIC_CIGAM64 : &'static [u8; 4] = b"\xcf\xfa\xed\xfe";


// this function can return an error if the file is smaller than the magic.
// Use the `is_macho_file` instead which does not fail which is actually
// much better for how this function is used within this library.
fn is_macho_file_as_result<P: AsRef<Path>>(path: P) -> CliResult<bool> {
    let mut f = File::open(&path)?;
    let mut magic : [u8; 4] = [0; 4];
    f.read_exact(&mut magic)?;
    Ok(match &magic {
        FAT_MAGIC | MAGIC | MAGIC_CIGAM | MAGIC_64 | MAGIC_CIGAM64 => true,
        _ => false
    })
}

pub fn is_macho_file<P: AsRef<Path>>(path: P) -> bool {
    is_macho_file_as_result(path).unwrap_or(false)
}
