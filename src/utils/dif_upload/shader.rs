//! Detection and metadata helpers for DXC shader debug files.
//!
//! Two kinds of files are handled here:
//!
//! - `dxil`: a DXIL container (`DXBC` magic). Its debug ID is the DXC hash
//!   encoded in the file stem, and its code ID is the NVIDIA Aftermath hash
//!   taken from a manifest written next to the files.
//! - `dxilpdb`: a DXC shader PDB. It carries the MSF magic but has no DBI
//!   stream, so symbolic cannot parse it. Its debug ID is also taken from the
//!   file stem.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use log::{debug, warn};
use symbolic::common::DebugId;

/// Name of the manifest file written by the Unreal cook hook.
pub const MANIFEST_FILE_NAME: &str = "sentry-shader-manifest.tsv";

const DXBC_MAGIC: &[u8] = b"DXBC";
const MSF_MAGIC: &[u8] = b"Microsoft C/C++ MSF 7.00\r\n\x1aDS";
const ILDN_MARKER: &[u8] = b"ILDN";
const ILDN_SCAN_LIMIT: usize = 64 * 1024;

/// Returns `true` if the buffer looks like a DXIL container.
pub fn is_dxil(data: &[u8]) -> bool {
    data.starts_with(DXBC_MAGIC)
}

/// Returns `true` if the buffer looks like a DXC shader PDB.
///
/// Such files start with the MSF magic and carry an `ILDN` part within the
/// first 64 KiB.
pub fn is_dxil_pdb(data: &[u8]) -> bool {
    if !data.starts_with(MSF_MAGIC) {
        return false;
    }

    let haystack = &data[..data.len().min(ILDN_SCAN_LIMIT)];
    haystack
        .windows(ILDN_MARKER.len())
        .any(|window| window == ILDN_MARKER)
}

/// Derives the debug ID from the file stem of a shader debug file.
///
/// DXC names shader artifacts after their 128-bit hash, encoded as 32 lowercase
/// hex characters. The hash is used verbatim as UUID with appendix 0.
pub fn debug_id_from_stem(name: &str) -> Option<DebugId> {
    let stem = Path::new(name).file_stem()?.to_str()?;
    if stem.len() != 32 || !stem.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }

    stem.parse().ok()
}

/// Metadata recorded for a single `.dxil` file in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestEntry {
    hash: u64,
    group: Option<String>,
}

/// Maps `.dxil` file names to their NVIDIA Aftermath shader hash and group.
#[derive(Debug, Default)]
pub struct ShaderManifest {
    entries: HashMap<String, ManifestEntry>,
}

impl ShaderManifest {
    /// Loads the manifest next to the given file, if present.
    ///
    /// Returns `None` if the directory contains no manifest or if it cannot be
    /// read.
    pub fn load_for(path: &Path) -> Option<Self> {
        let manifest_path = path.parent()?.join(MANIFEST_FILE_NAME);
        let contents = match fs::read_to_string(&manifest_path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
            Err(err) => {
                warn!(
                    "Could not read shader manifest {}: {err}",
                    manifest_path.display()
                );
                return None;
            }
        };

        let manifest = Self::parse(&contents);
        debug!(
            "loaded shader manifest {} with {} entries",
            manifest_path.display(),
            manifest.entries.len()
        );
        Some(manifest)
    }

    /// Parses manifest contents.
    ///
    /// Every line holds `<aftermath hash>\t<pdb name>\t<dxil path>[\t<group>]`.
    /// The optional group is trimmed; an empty group is treated as absent.
    /// Malformed lines are skipped.
    pub fn parse(contents: &str) -> Self {
        let entries = contents
            .lines()
            .filter_map(|line| {
                let mut columns = line.trim_end_matches('\r').split('\t');
                let hash = columns.next()?.trim().parse::<u64>().ok()?;
                let _pdb_name = columns.next()?;
                let dxil_name = file_name_of(columns.next()?)?;
                let group = columns
                    .next()
                    .map(str::trim)
                    .filter(|group| !group.is_empty())
                    .map(str::to_owned);
                Some((dxil_name.to_owned(), ManifestEntry { hash, group }))
            })
            .collect();

        Self { entries }
    }

    /// Returns the code ID for the given `.dxil` file name, if known.
    pub fn code_id(&self, dxil_file_name: &str) -> Option<String> {
        self.entries
            .get(dxil_file_name)
            .map(|entry| format!("{:016x}", entry.hash))
    }

    /// Returns the shader group for the given `.dxil` file name, if known.
    pub fn shader_group(&self, dxil_file_name: &str) -> Option<&str> {
        self.entries.get(dxil_file_name)?.group.as_deref()
    }
}

/// Returns the final component of a path that may use either separator.
fn file_name_of(path: &str) -> Option<&str> {
    let name = path
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .filter(|name| !name.is_empty())?;

    Path::new(name).file_name().and_then(OsStr::to_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dxil() {
        assert!(is_dxil(b"DXBC\x01\x02\x03"));
        assert!(!is_dxil(b"DXB"));
        assert!(!is_dxil(b"\x7fELF"));
    }

    #[test]
    fn test_is_dxil_pdb() {
        let mut data = MSF_MAGIC.to_vec();
        data.extend_from_slice(&[0; 4096]);
        assert!(!is_dxil_pdb(&data), "PDB without ILDN must not match");

        data.extend_from_slice(b"ILDN");
        assert!(is_dxil_pdb(&data));

        let mut far = MSF_MAGIC.to_vec();
        far.resize(ILDN_SCAN_LIMIT, 0);
        far.extend_from_slice(b"ILDN");
        assert!(!is_dxil_pdb(&far), "ILDN past 64 KiB must not match");

        assert!(!is_dxil_pdb(b"DXBCILDN"));
    }

    #[test]
    fn test_debug_id_from_stem() {
        let id = debug_id_from_stem("731322bf682bdc554eefdd89098b7383.dxil").unwrap();
        assert_eq!(id.to_string(), "731322bf-682b-dc55-4eef-dd89098b7383");
        assert_eq!(id.appendix(), 0);

        assert_eq!(
            debug_id_from_stem("sub/dir/731322bf682bdc554eefdd89098b7383.pdb"),
            Some(id)
        );
        assert!(debug_id_from_stem("731322bf682bdc554eefdd89098b738.dxil").is_none());
        assert!(debug_id_from_stem("not-a-hash.dxil").is_none());
        assert!(debug_id_from_stem("731322bf682bdc554eefdd89098b73zz.dxil").is_none());
    }

    #[test]
    fn test_manifest_parse() {
        let contents = "709220027517385083\t000057e2ed5c597bf3ca2651c6e13a1d.pdb\tD:\\x\\000057e2ed5c597bf3ca2651c6e13a1d.dxil\r\n\
                        14389219712596414370\t0001182625509522f9bcf6fce85c2ffc.pdb\tD:/y/0001182625509522f9bcf6fce85c2ffc.dxil\t\n\
                        123\t\n\
                        notanumber\tfoo.pdb\tfoo.dxil\n\
                        42\tbar.pdb\tbar.dxil\tSomeGroup\n";
        let manifest = ShaderManifest::parse(contents);

        assert_eq!(
            manifest.code_id("000057e2ed5c597bf3ca2651c6e13a1d.dxil"),
            Some("09d7a7db7fc4397b".to_owned())
        );
        assert_eq!(
            manifest.code_id("0001182625509522f9bcf6fce85c2ffc.dxil"),
            Some("c7b0c7175c5b8fa2".to_owned())
        );
        assert_eq!(
            manifest.code_id("bar.dxil"),
            Some("000000000000002a".to_owned())
        );
        assert_eq!(manifest.code_id("foo.dxil"), None);
        assert_eq!(manifest.code_id("missing.dxil"), None);
    }

    #[test]
    fn test_manifest_shader_group() {
        let contents = "1\ta.pdb\ta.dxil\n\
                        2\tb.pdb\tb.dxil\tGlobalShaders\n\
                        3\tc.pdb\tc.dxil\t  Padded Group \r\n\
                        4\td.pdb\td.dxil\t   \n\
                        5\te.pdb\te.dxil\t\n";
        let manifest = ShaderManifest::parse(contents);

        assert_eq!(manifest.shader_group("a.dxil"), None);
        assert_eq!(manifest.shader_group("b.dxil"), Some("GlobalShaders"));
        assert_eq!(manifest.shader_group("c.dxil"), Some("Padded Group"));
        assert_eq!(manifest.shader_group("d.dxil"), None);
        assert_eq!(manifest.shader_group("e.dxil"), None);
        assert_eq!(manifest.shader_group("missing.dxil"), None);

        assert_eq!(
            manifest.code_id("d.dxil"),
            Some("0000000000000004".to_owned())
        );
    }
}
