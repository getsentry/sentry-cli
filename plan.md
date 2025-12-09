# Plan: Manual Apple Crash Uploads via `sentry-cli`

## Goals
- Allow customers to upload raw macOS/iOS crash files (`.ips` JSON and plaintext `.crash`) directly to Sentry from `sentry-cli`.
- Reuse the existing `send-event`/`send-envelope` pipeline so crashes end up as standard error events inside envelopes that Relay already understands.
- Minimize assumptions about Relay accepting raw crash formats by transforming crashes into fully populated `SentryEvent`s client-side.

## Requirements & Constraints
1. Input: one or more crash files provided via path/glob, matching what `send-event` already accepts. Initial scope: `.ips` JSON (preferred) and classic `.crash` text. Fail fast with a helpful error if the format is unknown.
2. Output: for each crash, build one `Event` (or multiple if the file encodes multiple incidents) and send via an envelope to the DSN specified by the user (explicit flag) or their current auth context.
3. Feature should mirror UX of `send-event` (flags for release/dist/environment, attachments, ability to keep raw crash as attachment) and `send-envelope` (batching, globbing, streaming to `EnvelopesApi`).
4. Respect existing auth + rate limit behavior; no new ingestion endpoints.
5. Implementation must be deterministic/offline-friendly: no auto-download of dSYMs, rely on Sentry symbol server once events arrive.

## Proposed CLI UX
- New command `sentry-cli send-crash` (subcommand alias `upload-crash` if desired) registered beside `send-event` in `src/commands/mod.rs`.
- Flags:
  - `PATH` (required) accepting file or glob, same semantics as `send-envelope`.
  - `--dsn <DSN>` (optional); fallback to config/`SENTRY_DSN` like other commands.
  - `--platform` (default `apple-ios`), `--release`, `--dist`, `--env`, `--tags`, matching `send-event` for consistency.
  - `--attachment` or implicit: attach original crash file to envelope (enabled by default with opt-out `--no-attach` to avoid leaking sensitive data).
  - `--normalize-timestamps` bool to align to ISO times if needed (mirrors `send-event` timestamp logic).
  - Logging & dry-run flags from `send-event` if low cost.
- Output format identical to `send-event`: print event IDs per crash; show summary count + path for traceability.

## Architecture & Implementation Steps

### 1. Command scaffolding
- Add `send_crash.rs` mirroring `send_event.rs` structure.
- Register command in `src/commands/mod.rs` and CLI entrypoint.
- Wire ArgMatches to a new executor that loops over resolved file paths (use `glob_with` like other commands).

### 2. Crash format detection & loading
- New module `src/utils/apple_crash/mod.rs` with helpers:
  - `enum CrashDocument { Ips(IpsCrash), Crash(AppleTextCrash) }`.
  - `fn load_from_path(path: &Path) -> Result<Vec<CrashDocument>>` returning possibly multiple incidents (some `.ips` files contain an array under `diagnostics`).
  - Detection heuristic: inspect first non-whitespace char (`{` -> JSON). For `.crash`, rely on `Incident Identifier:` header.
  - Validate file size (e.g., < 10 MB) to avoid OOM, matching `send-event` safeguards.

### 3. IPS parser implementation
- Use `serde_json` to parse as `serde_json::Value` first, then map to strongly typed structs for the subset we need (metadata, threads, exception info, binary images, diagnostics).
- Follow Apple docs (`Interpreting the JSON Format of a Crash Report`). Define data structures (Incident, Threads, Frames) under `apple_crash::ips` module.
- Normalize optional fields (timestamps, OS version) into Rust types right away to avoid string juggling downstream.
- Expose conversion helpers: `impl From<IpsCrash> for NormalizedCrash` where `NormalizedCrash` is an internal struct used by both formats.

### 4. Text `.crash` parser implementation
- Reuse or adapt open-source parser? Options:
  - (Preferred) Implement lightweight parser using `nom`/manual scanning for the handful of sections Sentry needs (metadata, exception type, thread stacks, binary images). This keeps dependencies minimal and consistent with existing binary size constraints.
  - (Alternative) Vendor a tiny existing parser crate if license-compatible; evaluate during implementation.
- Parser should emit the same `NormalizedCrash` struct so downstream logic is format-agnostic.

### 5. Event conversion layering
- Create `NormalizedCrash -> Event` translator module: `fn build_event(crash: &NormalizedCrash, cli_opts: &CrashOpts) -> Event<'static>`.
- Reuse `send_event::send_raw_event` for final dispatch to benefit from `Client` defaults (SDK info, environment tags, release detection) and envelope creation.
- Populate key event fields:
  - `event.platform`: `"cocoa"`/`"apple"` depending on target, override via CLI flag.
  - `event.level`: default `error` unless the crash says otherwise.
  - `event.timestamp`: use crash occurrence time; fallback to file modified time.
  - Contexts: `device` (model, OS version), `os`, `app`, `battery`, etc., mapping from crash metadata.
  - `event.exception` with `values` representing the crash reason + signal, `mechanism` = `"mach"` or `"signal"`.
  - `event.threads` including stack frames; mark crashed thread.
  - `debug_meta` with binary image info, so Relay can symbolicate using uploaded dSYMs.
- For `.ips` files that package multiple crashes, emit one event per `diagnostics[i]` (loop accordingly).

### 6. Envelope construction & sending
- Rather than reconstructing envelope bytes manually, call `send_event::send_raw_event`. This returns the UUID and already sends via `EnvelopesApi` using configured DSN.
- If the DSN flag is provided, propagate it by extending `EnvelopesApi::try_new()` to accept overrides (similar to how other commands inject custom DSNs). This keeps envelope-building consistent with existing commands.
- If attachments enabled, wrap `Event` into an `Envelope` yourself: use `Envelope::from_event` helper, then `add_item` for attachment containing the raw crash file (content type `text/plain` or `application/json`). This matches how other Sentry SDKs send attachments.

### 7. User feedback & logging
- Print a summary after each file: `Crash from <path>#<index> dispatched: <event_id>`.
- Surface parse failures without aborting the entire batch (collect errors, continue). Provide `--fail-fast` flag if strict behavior desired.
- Hook into `log` macro (existing env var `SENTRY_LOG_LEVEL`) for verbose output referencing `send-event`.

### 8. Testing strategy
- Unit tests for parser modules using fixtures sampled from Apple docs and real-world `.ips`/`.crash` attachments (ensure PII-safe or scrubbed). Add them under `apple_crash/tests`.
- Integration tests (`tests/integration/send_crash.trycmd`) verifying CLI UX: load fixture, assert stdout contains event ID, and that HTTP request body matches expected event skeleton via mock server (reuse existing trycmd patterns for `send-event`).
- Add regression test that `--raw` fallback works: pass pre-built envelope containing Apple crash attachment to `send-envelope` to ensure compatibility.

### 9. Documentation & release readiness
- Update `README.md` and `docs/product/cli/send-event` section to mention new `send-crash` command, sample usage, and limitations.
- Add changelog entry under `Unreleased`.
- Coordinate with Product: ensure DSN requirement & permissions documented.

## Tradeoffs & Alternatives Considered
- **Server-side parsing vs CLI transformation**: implementing parsing in Relay would centralize logic but requires backend work + new ingestion endpoints. Client-side parsing lets us deliver value immediately and leverages existing envelope APIs, at the cost of duplicating parsing logic per client.
- **Full-fidelity parser vs minimal subset**: a full parser captures every section (memory, GPU, etc.) but increases implementation complexity. Plan opts for a `NormalizedCrash` capturing critical sections first, with room to extend later.
- **Reusing `send-event` envelope builder vs crafting envelopes manually**: reusing `send_event::send_raw_event` keeps behavior consistent and reduces risk. Manual envelopes would support attachments more directly but duplicate logic. Plan keeps the default path and only builds custom envelopes when attachments are requested.
- **Auto-upload dSYMs vs rely on existing pipeline**: automating dSYM lookup could improve symbolication but introduces dependency on build artifacts. Plan assumes dSYMs already uploaded, keeping feature focused on ingesting the crash artifact itself.

## Open Questions & Next Decisions
1. Do we also need to support zipped crash bundles or only raw files? (If yes, add unzip step before parsing.)
2. Should the CLI store derived events locally for auditing? (Potential future flag `--save-event`.)
3. Is attaching the raw crash mandatory for compliance reasons, or should it default to off to avoid uploading sensitive data? Confirmation needed before coding default behavior.
4. Are there Relay limitations when receiving attachments + constructed events that we need to coordinate with backend teams?

Once these questions are clarified, implementation can proceed following the steps above.
