//! WebAssembly debug-file helpers.
//!
//! Split/strip logic matches `wasm-split` from Symbolicator
//! (`crates/wasm-split`). That crate is a binary, not a library, so the
//! algorithm lives here and uses the same `wasmbin` types. Do not invent a
//! different split: the companion must keep the Code section (DWARF addresses
//! are relative to it) and both files must share a spec `build_id`.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use data_encoding::HEXLOWER;
use serde::Serialize;
use uuid::Uuid;
use wasmbin::io::{Decode as _, Encode as _};
use wasmbin::sections::{CustomSection, Section};
use wasmbin::Module;

/// How much debug information a WASM module actually contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugQuality {
    /// Embedded DWARF (`.debug_*` custom sections). Splittable.
    Dwarf,
    /// `external_debug_info` points at a separate debug file.
    ExternalDebugInfo,
    /// Name / symbol table only — function names, no file/line.
    Symtab,
    /// No debug information at all.
    None,
}

impl DebugQuality {
    pub fn has_dwarf(self) -> bool {
        matches!(self, Self::Dwarf | Self::ExternalDebugInfo)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dwarf => "dwarf",
            Self::ExternalDebugInfo => "external_debug_info",
            Self::Symtab => "symtab",
            Self::None => "none",
        }
    }
}

/// Inspected WASM module: section facts used to decide split vs skip.
#[derive(Debug)]
pub struct WasmInspection {
    pub quality: DebugQuality,
    pub build_id: Option<Vec<u8>>,
    pub has_code: bool,
    pub external_debug_info: Option<String>,
}

/// What `prepare` did (or would do) for one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepareAction {
    Split,
    WouldSplit,
    AlreadyPrepared,
    Skipped,
}

/// Outcome of preparing a single `.wasm` file.
#[derive(Debug, Serialize)]
pub struct PrepareResult {
    pub path: PathBuf,
    pub action: PrepareAction,
    pub quality: DebugQuality,
    pub build_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripped: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub companion: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl PrepareResult {
    fn skipped(path: PathBuf, quality: DebugQuality, warning: impl Into<String>) -> Self {
        Self {
            path,
            action: PrepareAction::Skipped,
            quality,
            build_id: None,
            stripped: None,
            companion: None,
            warning: Some(warning.into()),
        }
    }
}

/// Options for [`prepare_wasm_file`].
#[derive(Clone, Copy, Debug, Default)]
pub struct PrepareOptions<'a> {
    pub dry_run: bool,
    pub out_dir: Option<&'a Path>,
    pub build_id: Option<Uuid>,
}

pub fn is_wasm_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wasm"))
}

/// `foo.debug.wasm` companions produced by this command (and by `wasm-split`).
pub fn is_debug_companion_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.to_ascii_lowercase().ends_with(".debug.wasm"))
}

/// Companion path for `app.wasm` → `app.debug.wasm` (next to the input, or in `out_dir`).
pub fn companion_path(wasm_path: &Path, out_dir: Option<&Path>) -> PathBuf {
    let name = wasm_path.file_stem().map_or_else(
        || "module.debug.wasm".to_owned(),
        |stem| format!("{}.debug.wasm", stem.to_string_lossy()),
    );
    match out_dir {
        Some(dir) => dir.join(name),
        None => wasm_path.with_file_name(name),
    }
}

fn as_custom_section(section: &Section) -> Option<&CustomSection> {
    section.try_as()?.try_contents().ok()
}

/// Returns `true` if this section should be stripped.
///
/// Copied from `wasm-split`: only `.debug_*` (and optionally the name section).
/// Code, `build_id`, and `external_debug_info` stay.
fn is_strippable_section(section: &Section, strip_names: bool) -> bool {
    as_custom_section(section).is_some_and(|section| match section {
        CustomSection::Name(_) => strip_names,
        other => other.name().starts_with(".debug_"),
    })
}

pub fn inspect_module(module: &Module) -> WasmInspection {
    let mut has_dwarf = false;
    let mut has_name_section = false;
    let mut has_code = false;
    let mut build_id = None;
    let mut external_debug_info = None;

    for section in &module.sections {
        if matches!(section, Section::Code(_)) {
            has_code = true;
        }
        if let Some(custom) = as_custom_section(section) {
            match custom {
                CustomSection::BuildId(id) => build_id = Some(id.clone()),
                CustomSection::Name(_) => has_name_section = true,
                CustomSection::ExternalDebugInfo(url) => {
                    if let Ok(url) = url.try_contents() {
                        external_debug_info = Some(url.clone());
                    }
                }
                other if other.name().starts_with(".debug_") => has_dwarf = true,
                _ => {}
            }
        }
    }

    let quality = if has_dwarf {
        DebugQuality::Dwarf
    } else if external_debug_info.is_some() {
        DebugQuality::ExternalDebugInfo
    } else if has_name_section {
        DebugQuality::Symtab
    } else {
        DebugQuality::None
    };

    WasmInspection {
        quality,
        build_id,
        has_code,
        external_debug_info,
    }
}

pub fn decode_module(path: &Path) -> Result<Module> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    Module::decode(&mut BufReader::new(file))
        .with_context(|| format!("Failed to parse WASM module {}", path.display()))
}

fn encode_module(module: &Module, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }
    let file =
        File::create(path).with_context(|| format!("Failed to create {}", path.display()))?;
    module
        .encode(&mut BufWriter::new(file))
        .with_context(|| format!("Failed to write WASM module {}", path.display()))
}

fn format_build_id(build_id: &[u8]) -> String {
    HEXLOWER.encode(build_id)
}

fn resolve_external_debug_path(wasm_path: &Path, url: &str) -> Option<PathBuf> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return None;
    }
    let referenced = Path::new(url);
    if referenced.is_absolute() {
        Some(referenced.to_path_buf())
    } else {
        Some(
            wasm_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(referenced),
        )
    }
}

fn read_build_id(path: &Path) -> Result<Option<Vec<u8>>> {
    let module = decode_module(path)?;
    Ok(inspect_module(&module).build_id)
}

/// Split `input` the way `wasm-split <input> -d <companion> --strip` does.
///
/// Reimplemented here instead of shelling out to `wasm-split` because Symbolicator
/// ships it as a standalone binary, not a Rust library sentry-cli can depend on.
/// Behavior must match `wasm-split` exactly (see module docs).
///
/// Injects `build_id` if missing, writes the full module (Code + DWARF) to
/// `companion`, strips `.debug_*` from the deployable copy, and adds
/// `external_debug_info` pointing at the companion filename.
pub fn split_wasm(
    input: &Path,
    companion: &Path,
    stripped_out: &Path,
    build_id: Option<Uuid>,
) -> Result<Vec<u8>> {
    let mut module = decode_module(input)?;
    let inspection = inspect_module(&module);

    let build_id = match inspection.build_id {
        Some(existing) => existing,
        None => {
            let new_id = build_id.unwrap_or_else(Uuid::new_v4).as_bytes().to_vec();
            module
                .sections
                .push(CustomSection::BuildId(new_id.clone()).into());
            new_id
        }
    };

    // Write the companion first, while DWARF and Code are still in the module.
    encode_module(&module, companion)?;

    module
        .sections
        .retain(|section| !is_strippable_section(section, false));

    let debug_file_name = companion
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .context("Companion path has no file name")?;

    module
        .sections
        .push(CustomSection::ExternalDebugInfo(debug_file_name.into()).into());

    encode_module(&module, stripped_out)?;
    verify_split(stripped_out, companion, &build_id, inspection.has_code)?;
    Ok(build_id)
}

fn verify_split(
    stripped: &Path,
    companion: &Path,
    expected_build_id: &[u8],
    original_had_code: bool,
) -> Result<()> {
    let stripped_module = decode_module(stripped)?;
    let companion_module = decode_module(companion)?;
    let stripped_info = inspect_module(&stripped_module);
    let companion_info = inspect_module(&companion_module);

    match (&stripped_info.build_id, &companion_info.build_id) {
        (Some(a), Some(b)) if a == b && a == expected_build_id => {}
        _ => bail!(
            "build_id mismatch after split (stripped={}, companion={})",
            stripped.display(),
            companion.display()
        ),
    }

    if companion_info.quality != DebugQuality::Dwarf {
        bail!(
            "Companion {} is missing DWARF debug sections",
            companion.display()
        );
    }

    if original_had_code && !companion_info.has_code {
        bail!(
            "Companion {} is missing the Code section (required for DWARF address mapping)",
            companion.display()
        );
    }

    if stripped_info.quality == DebugQuality::Dwarf {
        bail!(
            "Stripped module {} still contains DWARF sections",
            stripped.display()
        );
    }

    Ok(())
}

/// Classify and optionally split one `.wasm` file.
///
/// Higher-level wrapper around [`split_wasm`] for the `debug-files prepare`
/// command: inspects debug quality, skips unsuitable inputs, detects already-
/// prepared modules, supports dry-run and `--out-dir`, and returns a structured
/// [`PrepareResult`] instead of just writing files.
pub fn prepare_wasm_file(path: &Path, options: PrepareOptions<'_>) -> Result<PrepareResult> {
    if is_debug_companion_path(path) {
        return Ok(PrepareResult::skipped(
            path.to_path_buf(),
            DebugQuality::Dwarf,
            "already a debug companion (*.debug.wasm); skipping".to_owned(),
        ));
    }

    let module = match decode_module(path) {
        Ok(module) => module,
        Err(err) => {
            return Ok(PrepareResult::skipped(
                path.to_path_buf(),
                DebugQuality::None,
                format!("not a valid WASM module: {err:#}"),
            ));
        }
    };

    let inspection = inspect_module(&module);
    let expected_companion = companion_path(path, options.out_dir);
    let stripped_out = match options.out_dir {
        Some(dir) => dir.join(path.file_name().unwrap_or(path.as_os_str())),
        None => path.to_path_buf(),
    };

    // Already split: stripped module + companion with the same build_id.
    if inspection.quality != DebugQuality::Dwarf {
        if let Some(url) = inspection.external_debug_info.as_deref() {
            if let Some(existing) = resolve_external_debug_path(path, url) {
                if existing.is_file() {
                    let companion_id = read_build_id(&existing).ok().flatten();
                    if companion_id.is_some() && companion_id == inspection.build_id {
                        return Ok(PrepareResult {
                            path: path.to_path_buf(),
                            action: PrepareAction::AlreadyPrepared,
                            quality: DebugQuality::ExternalDebugInfo,
                            build_id: inspection.build_id.as_deref().map(format_build_id),
                            stripped: Some(path.to_path_buf()),
                            companion: Some(existing),
                            warning: None,
                        });
                    }
                }
            }
        }

        if expected_companion.is_file() && inspection.build_id.is_some() {
            if let Ok(Some(companion_id)) = read_build_id(&expected_companion) {
                if Some(&companion_id) == inspection.build_id.as_ref() {
                    return Ok(PrepareResult {
                        path: path.to_path_buf(),
                        action: PrepareAction::AlreadyPrepared,
                        quality: inspection.quality,
                        build_id: Some(format_build_id(&companion_id)),
                        stripped: Some(path.to_path_buf()),
                        companion: Some(expected_companion),
                        warning: None,
                    });
                }
            }
        }
    }

    match inspection.quality {
        DebugQuality::Dwarf => {}
        DebugQuality::ExternalDebugInfo => {
            return Ok(PrepareResult::skipped(
                path.to_path_buf(),
                inspection.quality,
                "has external_debug_info but no local companion with matching build_id".to_owned(),
            ));
        }
        DebugQuality::Symtab => {
            return Ok(PrepareResult::skipped(
                path.to_path_buf(),
                inspection.quality,
                "no line-level symbolication (name/symtab only)".to_owned(),
            ));
        }
        DebugQuality::None => {
            // A build_id without debug sections means someone already stripped
            // this module, so re-splitting would overwrite a good companion
            // with an empty one. Without a build_id it was simply built
            // without debug info.
            let warning = if inspection.build_id.is_some() {
                "already stripped (build_id present, no debug sections); \
                 splitting would produce a useless companion"
            } else {
                "no debug information; rebuild with DWARF \
                 (Emscripten -g, wasm-pack dwarf-debug-info)"
            };
            return Ok(PrepareResult::skipped(
                path.to_path_buf(),
                inspection.quality,
                warning.to_owned(),
            ));
        }
    }

    if options.dry_run {
        return Ok(PrepareResult {
            path: path.to_path_buf(),
            action: PrepareAction::WouldSplit,
            quality: inspection.quality,
            build_id: inspection.build_id.as_deref().map(format_build_id),
            stripped: Some(stripped_out),
            companion: Some(expected_companion),
            warning: None,
        });
    }

    let build_id = split_wasm(path, &expected_companion, &stripped_out, options.build_id)?;

    Ok(PrepareResult {
        path: path.to_path_buf(),
        action: PrepareAction::Split,
        quality: inspection.quality,
        build_id: Some(format_build_id(&build_id)),
        stripped: Some(stripped_out),
        companion: Some(expected_companion),
        warning: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmbin::sections::RawCustomSection;

    fn dwarf_module() -> Module {
        Module {
            sections: vec![CustomSection::Other(RawCustomSection {
                name: ".debug_info".into(),
                data: vec![0, 1, 2, 3].into(),
            })
            .into()],
        }
    }

    fn name_only_module() -> Module {
        Module {
            sections: vec![CustomSection::Name(Default::default()).into()],
        }
    }

    fn empty_module() -> Module {
        Module { sections: vec![] }
    }

    /// Module that already went through a split: build_id, no debug sections.
    fn stripped_module() -> Module {
        Module {
            sections: vec![CustomSection::BuildId(vec![7; 16]).into()],
        }
    }

    fn write_module(dir: &Path, name: &str, module: &Module) -> PathBuf {
        let path = dir.join(name);
        encode_module(module, &path).unwrap();
        path
    }

    #[test]
    fn companion_name_is_predictable() {
        assert_eq!(
            companion_path(Path::new("dist/app.wasm"), None),
            PathBuf::from("dist/app.debug.wasm")
        );
        assert_eq!(
            companion_path(Path::new("pkg/demo_bg.wasm"), None),
            PathBuf::from("pkg/demo_bg.debug.wasm")
        );
        assert_eq!(
            companion_path(Path::new("app.wasm"), Some(Path::new("symbols"))),
            PathBuf::from("symbols/app.debug.wasm")
        );
    }

    #[test]
    fn debug_companion_path_detection() {
        assert!(is_debug_companion_path(Path::new("app.debug.wasm")));
        assert!(!is_debug_companion_path(Path::new("app.wasm")));
        assert!(!is_debug_companion_path(Path::new("demo_bg.wasm")));
    }

    #[test]
    fn classifies_dwarf_name_and_empty() {
        let dwarf = inspect_module(&dwarf_module());
        assert_eq!(dwarf.quality, DebugQuality::Dwarf);

        let names = inspect_module(&name_only_module());
        assert_eq!(names.quality, DebugQuality::Symtab);

        assert_eq!(inspect_module(&empty_module()).quality, DebugQuality::None);
    }

    #[test]
    fn split_injects_matching_build_id_and_keeps_dwarf_on_companion() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.wasm", &dwarf_module());
        let companion = dir.path().join("app.debug.wasm");
        let build_id = split_wasm(&input, &companion, &input, None).unwrap();

        let stripped = inspect_module(&decode_module(&input).unwrap());
        let debug = inspect_module(&decode_module(&companion).unwrap());

        assert_eq!(stripped.build_id.as_deref(), Some(build_id.as_slice()));
        assert_eq!(debug.build_id.as_deref(), Some(build_id.as_slice()));
        assert_eq!(debug.quality, DebugQuality::Dwarf);
        assert_ne!(stripped.quality, DebugQuality::Dwarf);
        assert_eq!(
            stripped.external_debug_info.as_deref(),
            Some("app.debug.wasm")
        );
    }

    #[test]
    fn prepare_skips_symtab_only() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "unity.wasm", &name_only_module());
        let result = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(result.action, PrepareAction::Skipped);
        assert_eq!(result.quality, DebugQuality::Symtab);
        assert!(result.warning.unwrap().contains("no line-level"));
        assert!(!companion_path(&input, None).exists());
    }

    #[test]
    fn prepare_warns_on_module_without_debug_info() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.wasm", &empty_module());
        let result = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(result.action, PrepareAction::Skipped);
        assert_eq!(result.quality, DebugQuality::None);
        assert!(result.warning.unwrap().contains("no debug information"));
        assert!(!companion_path(&input, None).exists());
    }

    #[test]
    fn prepare_skips_already_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.wasm", &stripped_module());
        let result = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(result.action, PrepareAction::Skipped);
        assert!(result.warning.unwrap().contains("already stripped"));
        assert!(!companion_path(&input, None).exists());
    }

    #[test]
    fn prepare_skips_re_split_of_prepared_pair() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.wasm", &dwarf_module());
        let first = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(first.action, PrepareAction::Split);

        let second = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(second.action, PrepareAction::AlreadyPrepared);
        assert_eq!(second.build_id, first.build_id);
    }

    #[test]
    fn dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.wasm", &dwarf_module());
        let result = prepare_wasm_file(
            &input,
            PrepareOptions {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.action, PrepareAction::WouldSplit);
        assert!(!companion_path(&input, None).exists());
    }

    #[test]
    fn skips_debug_companion_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_module(dir.path(), "app.debug.wasm", &dwarf_module());
        let result = prepare_wasm_file(&input, PrepareOptions::default()).unwrap();
        assert_eq!(result.action, PrepareAction::Skipped);
    }
}
