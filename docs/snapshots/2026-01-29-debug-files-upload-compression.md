# Debug files upload compression findings

## Conversation summary
- `debug-files upload` does not compress whole debug files; it uploads raw bytes
  in chunks and only applies per-chunk transport compression (gzip or
  uncompressed).
- There is no CLI option to compress entire debug files before upload.
- The symbolic crates (v12.16.3) do not expose a whole-file DIF compression API.
  They only decompress compressed sections or embedded payloads when reading,
  and provide ZIP creation for source bundles.
- `symsorter` in the symbolicator repo performs whole-file compression itself
  using zstd, with `-z/-zz/-zzz` mapping to zstd levels. Symbolic is used there
  only for parsing/iterating objects.
- A symsorter-compressed file passed to `sentry-cli debug-files upload` would be
  skipped because format detection relies on object-file magic bytes; zstd
  wrappers do not match any supported DIF format. There is no whole-file
  decompression step in the upload path. If the format check were bypassed, the
  raw compressed bytes would be uploaded (still subject to per-chunk transport
  compression).
- Extracting the symsorter compression logic into symbolic would likely require
  a small public helper in `symbolic-debuginfo` (or similar) that wraps zstd
  encoding, and then updating symsorter to call it while keeping the CLI's
  compression-level mapping in symsorter.

## Question
Does `debug-files upload` ever compress the entire debug file before uploading
(not just transport-level chunk compression)?

## Findings
- The `debug-files upload` command delegates to `DifUpload::upload` and does
  not apply any whole-file compression step to DIFs before upload.
  It scans files, builds `DifMatch` objects, and uploads them via the
  chunked upload path without wrapping the full DIF in a compressed container.
  (See `src/commands/debug_files/upload.rs:L225-L294`,
  `src/utils/dif_upload/mod.rs:L1166-L1204`.)
- Each DIF is chunked directly from its raw bytes. `DifMatch` implements
  `AsRef<[u8]>` by returning `data()`; `Chunked::from` computes checksums and
  splits those bytes into chunks. There is no full-file compression step here.
  (See `src/utils/dif_upload/mod.rs:L259-L262`,
  `src/utils/chunks/types.rs:L60-L73`.)
- Compression only happens per chunk when uploading: `Api::upload_chunks`
  compresses each chunk with `Api::compress` (gzip or uncompressed) and sends it
  as multipart data. This is transport-level compression, not a single
  compressed DIF.
  (See `src/api/mod.rs:L354-L392`.)

## Notes
- When `--include-sources` is used, sentry-cli generates source bundle archives
  via `SourceBundleWriter` and uploads those bundles as separate DIFs. This is
  creation of a new archive artifact, not compression of the original DIF
  itself. (See `src/utils/dif_upload/mod.rs:L1052-L1098`.)

## Symbolic crate findings (v12.16.3)
- The symbolic crate documentation focuses on symbolication and debug info
  parsing (object formats, symcache, minidump, etc.) and does not describe an
  API for compressing entire debug files. (See
  `/workspace/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/symbolic-12.16.3/README.md`
  and `src/lib.rs`.)
- `symbolic-debuginfo` includes **decompression** support for compressed data
  inside debug files (e.g., compressed DWARF sections and embedded portable PDB
  payloads), but this is for reading/expanding data on access, not for creating
  compressed DIFs. (See
  `symbolic-debuginfo-12.16.3/src/dwarf.rs:L182-L205`,
  `symbolic-debuginfo-12.16.3/src/elf.rs:L577-L626`,
  `symbolic-debuginfo-12.16.3/src/pe.rs:L502-L523`.)
- The only write-side “compression” in symbolic is for **source bundles**: the
  `SourceBundleWriter` builds a ZIP archive using `ZipWriter`, which is a
  separate artifact type and not a general-purpose DIF compressor. (See
  `symbolic-debuginfo-12.16.3/src/sourcebundle/mod.rs:L1076-L1139` and
  `L1114-L1121`.)

## Symsorter implementation (symbolicator repo)
- `symsorter` implements whole-file compression itself, using the `zstd` crate
  directly. The `--compress/-z` flag increments a `compression_level`, which is
  mapped to zstd levels and applied when writing each object to disk with
  `zstd::stream::copy_encode`. (See
  `symbolicator/crates/symsorter/src/app.rs`, `Cli::compression_level` and
  `process_file` where `copy_encode(obj.data(), &mut out, compression_level)` is
  called.)
- `symbolic` is used for parsing and iterating over objects (`Archive`, `Object`,
  `ObjectKind`), but compression is performed in symsorter, not via symbolic
  APIs. (See `symbolicator/crates/symsorter/src/app.rs` imports and usage.)

## CLI compression option
- `debug-files upload` does not expose a flag to compress whole debug files
  before upload. The upload path only applies per-chunk transport compression
  negotiated with the server. (See `src/commands/debug_files/upload.rs` and
  `src/api/mod.rs:L354-L392`.)

## Symsorter-compressed file uploaded via sentry-cli
- DIF discovery uses `Archive::peek` to identify object formats. If the format
  is `Unknown`, the file is skipped during collection. (See
  `src/utils/dif_upload/mod.rs:L733-L754`.)
- `Archive::peek` looks for known object magic bytes (ELF, PE, Mach-O, PDB,
  etc.) and does not recognize zstd wrappers. (See
  `symbolic-debuginfo-12.16.3/src/object.rs:L127-L172`.)
- There is no whole-file decompression step in the upload path, so a symsorter
  zstd-wrapped file would not be decompressed by sentry-cli. If the format check
  were bypassed, the raw compressed bytes would be uploaded.

## Extracting symsorter compression into symbolic (discussion)
- This would require adding a public helper (likely in `symbolic-debuginfo`) to
  zstd-compress raw bytes or write a compressed stream, then updating symsorter
  to call that helper.
- The policy mapping of `-z/-zz/-zzz` to zstd levels is CLI-specific and should
  remain in symsorter, while symbolic would accept a raw zstd level.
