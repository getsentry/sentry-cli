//! Detection and manifest lookup for DXC shader debug files.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::warn;

const MANIFEST_FILE_NAME: &str = "sentry-shader-manifest.tsv";
const MSF_MAGIC: &[u8] = b"Microsoft C/C++ MSF 7.00\r\n\x1aDS";
const ILDN_SCAN_LIMIT: usize = 64 * 1024;

/// Returns `true` if the buffer is a DXIL container.
pub fn is_dxil(data: &[u8]) -> bool {
    data.starts_with(b"DXBC")
}

/// Returns `true` if the buffer is a DXC shader PDB.
///
/// These carry the MSF magic but no DBI stream, so symbolic cannot parse them.
/// They are told apart from regular PDBs by the `ILDN` part near the start of
/// the file.
pub fn is_dxil_pdb(data: &[u8]) -> bool {
    data.starts_with(MSF_MAGIC)
        && data[..data.len().min(ILDN_SCAN_LIMIT)]
            .windows(4)
            .any(|window| window == b"ILDN")
}

/// Manifest metadata of a single `.dxil` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderInfo {
    /// NVIDIA Aftermath shader hash as 16 hex digits.
    pub code_id: String,
    pub shader_group: Option<String>,
}

/// `.dxil` file names mapped to their manifest metadata.
pub type ShaderManifest = HashMap<String, ShaderInfo>;

/// Loads the manifest written next to the shader files in `dir`.
pub fn load_manifest(dir: &Path) -> ShaderManifest {
    let path = dir.join(MANIFEST_FILE_NAME);
    match fs::read_to_string(&path) {
        Ok(contents) => parse_manifest(&contents),
        Err(err) => {
            warn!(
                "Could not read {}: {err}; DXIL files are uploaded without code ID",
                path.display()
            );
            ShaderManifest::new()
        }
    }
}

/// Parses lines of the form `<aftermath hash>\t<pdb name>\t<dxil path>[\t<shader group>]`.
fn parse_manifest(contents: &str) -> ShaderManifest {
    contents
        .lines()
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let hash: u64 = columns.next()?.parse().ok()?;
            let _pdb_name = columns.next()?;
            let dxil_name = columns.next()?.rsplit(['\\', '/']).next()?;
            let shader_group = columns
                .next()
                .map(str::trim)
                .filter(|group| !group.is_empty())
                .map(str::to_owned);
            let info = ShaderInfo {
                code_id: format!("{hash:016x}"),
                shader_group,
            };
            Some((dxil_name.to_owned(), info))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dxil() {
        assert!(is_dxil(b"DXBC\x01\x02\x03"));
        assert!(!is_dxil(b"\x7fELF"));
    }

    #[test]
    fn test_is_dxil_pdb() {
        let mut data = MSF_MAGIC.to_vec();
        data.extend_from_slice(&[0; 4096]);
        assert!(!is_dxil_pdb(&data));

        data.extend_from_slice(b"ILDN");
        assert!(is_dxil_pdb(&data));

        let mut far = MSF_MAGIC.to_vec();
        far.resize(ILDN_SCAN_LIMIT, 0);
        far.extend_from_slice(b"ILDN");
        assert!(!is_dxil_pdb(&far));

        assert!(!is_dxil_pdb(b"DXBCILDN"));
    }

    #[test]
    fn test_parse_manifest() {
        let manifest = parse_manifest(
            "709220027517385083\t000057e2ed5c597bf3ca2651c6e13a1d.pdb\tD:\\x\\000057e2ed5c597bf3ca2651c6e13a1d.dxil\r\n\
             2\tb.pdb\tD:/y/b.dxil\tGlobalShaders\n\
             3\tc.pdb\tc.dxil\t  Padded Group \r\n\
             4\td.pdb\td.dxil\t   \n\
             123\t\n\
             notanumber\tfoo.pdb\tfoo.dxil\n",
        );

        assert_eq!(
            manifest["000057e2ed5c597bf3ca2651c6e13a1d.dxil"],
            ShaderInfo {
                code_id: "09d7a7db7fc4397b".into(),
                shader_group: None,
            }
        );
        assert_eq!(
            manifest["b.dxil"],
            ShaderInfo {
                code_id: "0000000000000002".into(),
                shader_group: Some("GlobalShaders".into()),
            }
        );
        assert_eq!(
            manifest["c.dxil"].shader_group.as_deref(),
            Some("Padded Group")
        );
        assert_eq!(manifest["d.dxil"].shader_group, None);
        assert_eq!(manifest.len(), 4);
    }
}
