# Debug files upload compression findings

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
