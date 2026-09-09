use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::Path;

use crate::integration::TestManager;

/// Minimal WASM with a `.debug_info` custom section.
const WASM_WITH_DWARF: &[u8] = b"\0asm\x01\x00\x00\x00\x00\x10\x0b.debug_info\x00\x01\x02\x03";

/// Minimal WASM with only a `name` custom section (symtab / no DWARF).
const WASM_NAME_ONLY: &[u8] = b"\0asm\x01\x00\x00\x00\x00\x05\x04name";

/// WASM header only — no debug sections and no build_id.
const WASM_EMPTY: &[u8] = b"\0asm\x01\x00\x00\x00";

/// WASM with a `build_id` custom section but no debug sections, i.e. a module
/// that already went through a split.
fn wasm_stripped_with_build_id() -> Vec<u8> {
    let mut bytes = b"\0asm\x01\x00\x00\x00\x00\x1a\x08build_id\x10".to_vec();
    bytes.extend([7u8; 16]);
    bytes
}

fn reset_dir(path: &str) {
    let path = Path::new(path);
    if path.exists() {
        remove_dir_all(path).unwrap();
    }
    create_dir_all(path).unwrap();
}

fn write_wasm(dir: &str, name: &str, bytes: &[u8]) {
    write(Path::new(dir).join(name), bytes).unwrap();
}

#[test]
fn command_debug_files_prepare_missing_path() {
    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-missing-path.trycmd");
}

#[test]
fn command_debug_files_prepare_dry_run() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-dry-run.in/";
    reset_dir(cwd);
    write_wasm(cwd, "app.wasm", WASM_WITH_DWARF);

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-dry-run.trycmd");
}

#[test]
fn command_debug_files_prepare_split_no_upload() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-no-upload.in/";
    reset_dir(cwd);
    write_wasm(cwd, "app.wasm", WASM_WITH_DWARF);

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-no-upload.trycmd");

    let companion = Path::new(cwd).join("app.debug.wasm");
    assert!(
        companion.is_file(),
        "expected companion {}",
        companion.display()
    );
}

#[test]
fn command_debug_files_prepare_symtab_warning() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-symtab.in/";
    reset_dir(cwd);
    write_wasm(cwd, "unity.wasm", WASM_NAME_ONLY);

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-symtab.trycmd");
}

#[test]
fn command_debug_files_prepare_require_dwarf() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-require-dwarf.in/";
    reset_dir(cwd);
    write_wasm(cwd, "unity.wasm", WASM_NAME_ONLY);

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-require-dwarf.trycmd");
}

#[test]
fn command_debug_files_prepare_no_debug_info() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-no-debug-info.in/";
    reset_dir(cwd);
    write_wasm(cwd, "app.wasm", WASM_EMPTY);

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-no-debug-info.trycmd");
}

#[test]
fn command_debug_files_prepare_already_stripped() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-stripped.in/";
    reset_dir(cwd);
    write_wasm(cwd, "app.wasm", &wasm_stripped_with_build_id());

    TestManager::new()
        .register_trycmd_test("debug_files/prepare/debug_files-prepare-stripped.trycmd");
}

#[test]
fn command_debug_files_prepare_json() {
    let cwd = "tests/integration/_cases/debug_files/prepare/debug_files-prepare-json.in/";
    reset_dir(cwd);
    write_wasm(cwd, "app.wasm", WASM_WITH_DWARF);

    TestManager::new().register_trycmd_test("debug_files/prepare/debug_files-prepare-json.trycmd");
}

#[test]
fn command_debug_files_prepare_help() {
    TestManager::new().register_trycmd_test("debug_files/prepare/debug_files-prepare-help.trycmd");
}
