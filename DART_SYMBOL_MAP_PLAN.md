## Implementation Plan: upload-dart-symbol-map

Goal: Add a CLI command to upload a Dart/Flutter symbol map ("dartsymbolmap") for deobfuscating Dart exception types. The upload must use the chunk-upload flow and associate the mapping to a required debug id, preferably extracted from an associated debug file.

### High-level architecture (aligned with existing codebase)
- Reuse the existing chunked upload infrastructure in `src/utils/chunks/` and the assemble endpoint in `src/api/mod.rs::assemble_difs(...)`.
- Introduce a small wrapper object implementing `Assemblable` for the mapping file so it can be sent through `chunks::upload_chunked_objects`.
- Add a new CLI subcommand, following patterns used by `upload_proguard` and `sourcemaps upload`.
- Validate the mapping file locally before upload to fail fast.
- Ensure server capability support via `ChunkServerOptions.accept` for a new capability string "dartsymbolmap".

---

### Step-by-step tasks

1) Capability support for "dartsymbolmap"
   - Add a new variant to `ChunkUploadCapability` and update deserialization:
     - File: `src/api/data_types/chunking/upload/capability.rs`
     - Add `DartSymbolMap` variant and map the string `"dartsymbolmap"` to it in the `Deserialize` impl.
   - No change needed to `ChunkServerOptions.should_strip_debug_ids()`.
   - Optional: Add a helper method (if desired) to check support: `options.supports(ChunkUploadCapability::DartSymbolMap)`.

2) Represent the mapping to be uploaded
   - Create a lightweight struct, e.g. `DartSymbolMapObject { bytes: Vec<u8>, name: String, debug_id: DebugId }`.
   - Implement:
     - `AsRef<[u8]>` to expose raw bytes
     - `Assemblable` to provide `name()` and `debug_id()` (see `src/utils/chunks/types.rs`)
     - `Display` for user-friendly printing during summaries
   - We will then wrap it with `chunks::Chunked::from(object, chunk_size)` so we can reuse `chunks::upload_chunked_objects(...)` which:
     - Calls `assemble_difs` to retrieve missing chunks and current states
     - Uploads missing chunks via `Api::upload_chunks(...)`
     - Polls assemble until completion (configurable via `ChunkOptions::with_max_wait`)

3) Local validation of the Dart symbol map file
   - Implement a small validator function used by the command before uploading:
     - Read file bytes; parse JSON as `Vec<String>` using `serde_json`.
     - Ensure even length; if odd or not an array of strings, error with a clear message.
   - Keep the file name as provided; if the user passes a directory name or odd extension, continue but recommend `dartsymbolmap.json` in help text.

4) Debug ID resolution
   - Inputs: a required path to the associated debug file (not the dartsymbolmap).
   - Behavior:
     - Open the associated debug file and extract debug ids via `utils::dif::DifFile::open_path(...).ids()`.
     - If exactly one id is present, use it.
     - If multiple, error and ask the user to disambiguate (e.g., select the correct variant via a future flag) — for now we will error with a clear message.
     - If none, error out with a clear message.

5) New CLI command: upload-dart-symbol-map
   - File: `src/commands/upload_dart_symbol_map.rs`
   - Command shape (similar to `upload_proguard.rs`):
     - `.about("Upload a Dart symbol map file to a project.")`
     - `.org_arg()` and `.project_arg(false)` and fetch both via `Config::current().get_org_and_project(matches)?`.
     - Positional args:
       - `mapping` (required): path to `dartsymbolmap.json`
       - `debug_file` (required): path to the associated debug file to extract the debug id
     - Flags: none (keep command simple; no wait flags)
   - Execution flow:
     1. Validate mapping file (JSON array of strings, even length)
     2. Resolve `debug_id` as described in Step 4
     3. Get `ChunkServerOptions` via `api.authenticated()?.get_chunk_upload_options(&org)?`
     4. Ensure `options.supports(ChunkUploadCapability::DartSymbolMap)`, else bail with an actionable message
     5. Build `DartSymbolMapObject` with bytes, `name` (use basename of path; recommend `dartsymbolmap.json`), and `debug_id`
     6. Compute chunking: `Chunked::from(object, options.chunk_size as usize)`
      7. Construct `ChunkOptions::new(options, org, project).with_max_wait(DEFAULT_MAX_WAIT)` to always wait/poll until completion (bounded by server `max_wait`)
     8. Call `chunks::upload_chunked_objects(&[chunked], chunk_options)`
     9. Rely on existing summary output from `chunks::upload_chunked_objects`/`poll_assemble`; optionally print a concise success line for the mapping

6) Wire into the root command
   - File: `src/commands/mod.rs`
     - Add `mod upload_dart_symbol_map;`
     - Add to `each_subcommand!` list so it’s registered
     - Optional: consider adding to `UPDATE_NAGGER_CMDS`

7) Tests (integration-focused)
   - Directory: `tests/integration/_cases/dart_symbol_map/`
   - Cases:
     - `help.trycmd`: validates help text
     - `validate-json-invalid.trycmd`: invalid JSON (non-array) -> error
     - `validate-json-odd-length.trycmd`: odd number of elements -> error
      - `missing-debug-id.trycmd`: debug file has no debug id -> error
      - `multiple-debug-ids.trycmd`: ambiguous debug file -> error with guidance to disambiguate (no `--debug-id` supported)
     - `happy-path.trycmd`: mock chunk-upload+assemble:
       - GET chunk-upload returns accept includes `"dartsymbolmap"` and compression includes `gzip`
       - First assemble returns `NOT_FOUND` with `missingChunks`
       - Upload chunks
       - Subsequent assemble returns `OK` (or `CREATED` → subsequent `OK`)
   - Fixtures:
     - `tests/integration/_fixtures/dartsymbolmap.json`
     - Sample debug file already in `_fixtures/` (Mach-O, ELF, Breakpad, etc.)
   - Mock responses live under `tests/integration/_responses/`

8) Documentation and UX
   - Add command description and examples to `--help` and a short note in `README.md`.
   - Clear errors for capability missing, invalid mapping, and debug id issues.

9) Non-functional notes
   - Hash algorithm: server specifies `sha1`; existing utilities already use SHA1 (see `get_sha1_checksums`).
   - Region correctness: the `url` from chunk-upload options is used directly by `Api::upload_chunks`.
   - Compression: chosen automatically from `ChunkServerOptions.compression` (prefers `gzip` when supported).

---

### Key code references
- Chunk server options and assemble:
  - `src/api/mod.rs::get_chunk_upload_options`, `assemble_difs` (POST `/files/difs/assemble/`)
- Chunking utilities and traits:
  - `src/utils/chunks/` (`ChunkOptions`, `Chunked`, `Assemblable`, `upload_chunked_objects`)
- Capability parsing:
  - `src/api/data_types/chunking/upload/capability.rs`
- Chunk-upload network call (multipart with filename = chunk sha1):
  - `src/api/mod.rs::upload_chunks`
- Debug-id extraction from debug files:
  - `src/utils/dif.rs` (`DifFile::open_path`, `ids()`)

---

### Assumptions & open questions
- Server side recognizes `name` and `debug_id` for `dartsymbolmap` via `assemble_difs`. No extra type discriminator required.
- Association rule: the mapping’s `debug_id` must match the associated debug file’s id; we will enforce extraction exclusively from the provided debug file path (no `--debug-id` override).
- If you want the command to accept a single path and infer the debug file in the future, we can extend UX later, but for now two paths are required.

---

### Rollout checklist
1. Implement code changes above
2. `cargo build --workspace`
3. `cargo test --workspace` (run with `TRYCMD=overwrite` once to create snapshots)
4. Verify on a project with a real dartsymbolmap and debug file
5. Add brief docs snippet to README


