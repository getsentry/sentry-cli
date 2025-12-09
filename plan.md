# Implementation Plan: Send Apple Crash Reports (.ips) to Sentry

## 📋 Overview

This plan details the implementation of a new `send-apple-crash` command for sentry-cli that allows users to manually upload Apple crash reports in `.ips` (JSON) format to Sentry. This addresses the use case where users receive crash reports directly (e.g., from App Store Review rejections or corporate environments with restricted network access) and want to leverage Sentry's symbolication capabilities.

## 🎯 Goals

1. Create a new `send-apple-crash` command that accepts `.ips` files
2. Parse the `.ips` JSON format into a Sentry Event structure
3. Send the event to Sentry via an envelope (similar to `send-event`)
4. Let Sentry's backend handle symbolication (no local symbolication)
5. Support both single files and glob patterns for batch uploads

## 📚 Background Context

### What are .ips files?
- `.ips` files are Apple's JSON-format crash reports introduced with Xcode
- Documented at: https://developer.apple.com/documentation/xcode/interpreting-the-json-format-of-a-crash-report
- Contains crash metadata, exception info, thread stack traces, binary images, and device information

### Use Cases
1. **App Store Review rejections**: Apple provides `.ips` files when apps crash during review
2. **Corporate environments**: Users behind restrictive firewalls manually provide crash reports
3. **Manual testing**: QA teams collecting crashes from test devices

### Existing Similar Commands
- `send-event`: Sends manually created events (JSON format) to Sentry
- `send-envelope`: Sends pre-formatted envelopes to Sentry
- Both use `EnvelopesApi::send_envelope()` to dispatch to the DSN

## 🏗️ Architecture Design

### Command Flow
```
User runs: sentry-cli send-apple-crash crash.ips
    ↓
Parse .ips JSON file
    ↓
Extract crash data into Sentry Event structure
    ↓
Create envelope with event
    ↓
Send to DSN via EnvelopesApi
    ↓
Report success/failure
```

### Key Components
1. **Command Module**: `src/commands/send_apple_crash.rs`
2. **Parser Module**: `src/utils/apple_crash.rs` (new utility)
3. **API Integration**: Use existing `EnvelopesApi`
4. **Tests**: Integration tests in `tests/integration/send_apple_crash.rs`

## 📝 Detailed Implementation Steps

### Step 1: Create the Parser Module

**File**: `src/utils/apple_crash.rs`

**Purpose**: Parse `.ips` JSON format and convert to Sentry Event

**Key Functions**:

```rust
// Main parsing function
pub fn parse_ips_crash_report(content: &str) -> Result<Event<'static>>

// Helper to extract exception information
fn extract_exception(crash_json: &Value) -> Result<Vec<Exception>>

// Helper to extract thread information
fn extract_threads(crash_json: &Value) -> Result<Vec<Thread>>

// Helper to extract device/app context
fn extract_contexts(crash_json: &Value) -> Result<BTreeMap<String, Context>>

// Helper to extract binary images (for symbolication)
fn extract_debug_images(crash_json: &Value) -> Result<Vec<DebugImage>>
```

**IPS JSON Structure to Parse**:

Based on Apple's documentation, key fields include:

- `exception` → Maps to Sentry `Exception`
  - `type`: Exception type (e.g., "EXC_BAD_ACCESS")
  - `signal`: Signal name (e.g., "SIGSEGV")
  - `codes`: Exception codes
  - `subtype`: Additional exception info

- `threads` → Maps to Sentry `Stacktrace` and `Thread`
  - `id`: Thread ID
  - `name`: Thread name (optional)
  - `crashed`: Boolean indicating crashed thread
  - `frames`: Array of stack frames
    - `imageOffset`: Offset in binary
    - `imageIndex`: Index into binary images array
    - `symbol`: Symbol name (if available)
    - `symbolLocation`: Offset from symbol

- `usedImages` → Maps to Sentry `DebugImage`
  - `uuid`: Debug identifier
  - `name`: Binary name/path
  - `arch`: Architecture (e.g., "arm64")
  - `base`: Load address

- Metadata (optional, for context):
  - `incident`: Incident identifier
  - `crashReporterKey`: Device identifier
  - `osVersion`: OS version string
  - `appVersion`: App version
  - `bundleID`: Bundle identifier

**Sentry Event Structure**:

Create a `sentry::protocol::Event` with:
- `platform`: "cocoa" (for Apple platforms)
- `level`: "error" or "fatal"
- `exception`: Parsed exception information
- `threads`: Thread information with stack traces
- `debug_meta`: Debug images for symbolication
- `contexts`: Device, OS, and app information
- `timestamp`: From crash report or current time
- `release`: From app version (if available)
- `environment`: Optional (from CLI args)
- `sdk`: SDK info (mark as sentry-cli)

**Error Handling**:
- Invalid JSON → Return error with helpful message
- Missing required fields → Use sensible defaults where possible
- Malformed stack traces → Skip frames that can't be parsed

### Step 2: Create the Command Module

**File**: `src/commands/send_apple_crash.rs`

**Command Definition**:

```rust
pub fn make_command(command: Command) -> Command {
    command
        .about("Send Apple crash reports to Sentry.")
        .long_about(
            "Send Apple crash reports (.ips) to Sentry.{n}{n}\
             This command parses Apple crash report files in .ips (JSON) format \
             and sends them to Sentry as error events. Sentry will automatically \
             symbolicate the crash reports if matching debug symbols (dSYMs) have \
             been uploaded.{n}{n}\
             Due to network errors, rate limits or sampling the event is not guaranteed to \
             actually arrive. Check debug output for transmission errors by passing --log-level=\
             debug or setting `SENTRY_LOG_LEVEL=debug`.",
        )
        .arg(
            Arg::new("path")
                .value_name("PATH")
                .required(true)
                .help("The path or glob to .ips file(s) to send as crash events."),
        )
        .arg(
            Arg::new("release")
                .value_name("RELEASE")
                .long("release")
                .short('r')
                .help("Optional release identifier to associate with the crash."),
        )
        .arg(
            Arg::new("environment")
                .value_name("ENVIRONMENT")
                .long("env")
                .short('E')
                .help("Optional environment name (e.g., production, staging)."),
        )
        .arg(
            Arg::new("dist")
                .value_name("DISTRIBUTION")
                .long("dist")
                .short('d')
                .value_parser(validate_distribution)
                .help("Optional distribution identifier."),
        )
}
```

**Execution Logic**:

```rust
pub fn execute(matches: &ArgMatches) -> Result<()> {
    let path = matches.get_one::<String>("path").unwrap();
    
    // Collect paths using glob (supports wildcards like *.ips)
    let collected_paths: Vec<PathBuf> = glob_with(path, MatchOptions::new())
        .context("Invalid glob pattern")?
        .flatten()
        .collect();

    if collected_paths.is_empty() {
        warn!("Did not match any .ips files for pattern: {path}");
        return Ok(());
    }

    // Process each crash file
    for path in collected_paths {
        let content = fs::read_to_string(&path)
            .context(format!("Failed to read crash file: {}", path.display()))?;
        
        // Parse the .ips file into a Sentry event
        let mut event = parse_ips_crash_report(&content)
            .context(format!("Failed to parse crash file: {}", path.display()))?;
        
        // Override with CLI arguments if provided
        if let Some(release) = matches.get_one::<String>("release") {
            event.release = Some(Cow::Owned(release.clone()));
        }
        if let Some(environment) = matches.get_one::<String>("environment") {
            event.environment = Some(Cow::Owned(environment.clone()));
        }
        if let Some(dist) = matches.get_one::<String>("dist") {
            event.dist = Some(Cow::Owned(dist.clone()));
        }
        
        // Send the event
        let event_id = send_raw_event(event)?;
        println!("Crash from file {} dispatched: {}", path.display(), event_id);
    }

    Ok(())
}

// Reuse the send_raw_event function from send_event module
// or implement a similar version here
fn send_raw_event(event: Event<'static>) -> Result<Uuid> {
    let client = Client::from_config(apply_defaults(ClientOptions {
        user_agent: USER_AGENT.into(),
        ..Default::default()
    }));
    let event = client
        .prepare_event(event, None)
        .ok_or(anyhow!("Event dropped during preparation"))?;
    let event_id = event.event_id;
    EnvelopesApi::try_new()?.send_envelope(event)?;
    Ok(event_id)
}
```

### Step 3: Register the Command

**File**: `src/commands/mod.rs`

Add the module and register it:

```rust
// Add to module declarations
mod send_apple_crash;

// Add to each_subcommand! macro
$mac!(send_apple_crash);
```

### Step 4: Add Dependency Declarations

**File**: `src/utils/mod.rs`

Add the new utility module:

```rust
pub mod apple_crash;
```

### Step 5: Create Test Fixtures

**Directory**: `tests/integration/_fixtures/`

Create test `.ips` files:

1. **`crash_simple.ips`**: Minimal valid crash report
   - Single thread crash
   - Basic exception information
   - One or two binary images

2. **`crash_complete.ips`**: Full crash report
   - Multiple threads
   - Complete metadata
   - Full device/OS information
   - Multiple binary images

3. **`crash_invalid.json`**: Invalid JSON (for error testing)

4. **`crash_malformed.ips`**: Valid JSON but missing required crash fields

Example minimal crash fixture structure:
```json
{
  "incident": "A1B2C3D4-1234-5678-9ABC-DEF012345678",
  "crashReporterKey": "test-device-key",
  "osVersion": "iOS 17.0 (21A329)",
  "bundleID": "io.sentry.test",
  "appVersion": "1.0.0",
  "exception": {
    "type": "EXC_BAD_ACCESS",
    "signal": "SIGSEGV",
    "codes": "0x0000000000000001, 0x0000000000000000",
    "subtype": "KERN_INVALID_ADDRESS at 0x0000000000000000"
  },
  "threads": [{
    "id": 0,
    "crashed": true,
    "frames": [{
      "imageOffset": 4096,
      "imageIndex": 0,
      "symbol": "main",
      "symbolLocation": 0
    }]
  }],
  "usedImages": [{
    "uuid": "12345678-1234-1234-1234-123456789abc",
    "name": "TestApp",
    "arch": "arm64",
    "base": 4294967296
  }]
}
```

### Step 6: Create Integration Tests

**File**: `tests/integration/send_apple_crash.rs`

```rust
use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_send_apple_crash() {
    TestManager::new()
        .mock_endpoint(MockEndpointBuilder::new("POST", "/api/1337/envelope/"))
        .register_trycmd_test("send_apple_crash/*.trycmd");
}

#[test]
fn command_send_apple_crash_invalid() {
    // Tests for error cases
    TestManager::new()
        .register_trycmd_test("send_apple_crash/error/*.trycmd");
}
```

**Directory**: `tests/integration/_cases/send_apple_crash/`

Create `.trycmd` test files:

1. **`send_apple_crash-help.trycmd`**: Test help output
```
$ sentry-cli send-apple-crash --help
? success
...
```

2. **`send_apple_crash-file.trycmd`**: Test basic crash upload
```
$ sentry-cli send-apple-crash tests/integration/_fixtures/crash_simple.ips
? success
Crash from file tests/integration/_fixtures/crash_simple.ips dispatched: [..]
```

3. **`send_apple_crash-glob.trycmd`**: Test glob pattern
```
$ sentry-cli send-apple-crash "tests/integration/_fixtures/crash_*.ips"
? success
Crash from file tests/integration/_fixtures/crash_simple.ips dispatched: [..]
Crash from file tests/integration/_fixtures/crash_complete.ips dispatched: [..]
```

4. **`send_apple_crash-with-release.trycmd`**: Test with release flag
```
$ sentry-cli send-apple-crash --release 1.2.3 tests/integration/_fixtures/crash_simple.ips
? success
Crash from file tests/integration/_fixtures/crash_simple.ips dispatched: [..]
```

5. **`error/send_apple_crash-invalid.trycmd`**: Test invalid JSON
```
$ sentry-cli send-apple-crash tests/integration/_fixtures/crash_invalid.json
? failed
Error: Failed to parse crash file: tests/integration/_fixtures/crash_invalid.json
...
```

6. **`error/send_apple_crash-no-file.trycmd`**: Test missing file
```
$ sentry-cli send-apple-crash nonexistent.ips
? failed
...
```

### Step 7: Integration Test Setup

**File**: `tests/integration/mod.rs`

Add the test module:

```rust
mod send_apple_crash;
```

## 🔍 Implementation Details

### IPS to Sentry Event Mapping

| IPS Field | Sentry Event Field | Notes |
|-----------|-------------------|-------|
| `exception.type` | `exception.type` | Exception type string |
| `exception.signal` | `exception.mechanism.meta.signal.name` | Signal name |
| `exception.codes` | `exception.value` | Human-readable description |
| `threads[].frames` | `threads[].stacktrace.frames` | Convert frame format |
| `threads[].crashed` | Mark crashed thread in stacktrace | |
| `usedImages` | `debug_meta.images` | Debug symbols for symbolication |
| `osVersion` | `contexts.os.name` and `version` | Parse OS string |
| `appVersion` | `release` | If not overridden by CLI |
| `bundleID` | `contexts.app.app_identifier` | |
| `crashReporterKey` | `contexts.device.id` | Device identifier |

### Frame Format Conversion

IPS frame structure:
```json
{
  "imageOffset": 4096,
  "imageIndex": 0,
  "symbol": "main",
  "symbolLocation": 0
}
```

Sentry frame structure:
```rust
Frame {
    instruction_addr: Some(HexValue(base_addr + imageOffset)),
    package: Some(image_name),
    symbol: symbol (if present),
    function: symbol (if present),
    image_addr: Some(HexValue(base_addr)),
    ..Default::default()
}
```

### Debug Image Format Conversion

IPS binary image:
```json
{
  "uuid": "12345678-1234-1234-1234-123456789abc",
  "name": "/path/to/binary",
  "arch": "arm64",
  "base": 4294967296
}
```

Sentry debug image:
```rust
DebugImage::Apple(AppleDebugImage {
    uuid: DebugId::from_uuid(parse_uuid(uuid)?),
    image_addr: HexValue(base),
    image_size: None, // Not available in .ips
    image_vmaddr: None,
    name: name.into(),
    arch: Some(arch.into()),
    ..Default::default()
})
```

### Error Handling Strategy

1. **File Not Found**: Return clear error with file path
2. **Invalid JSON**: Return JSON parsing error with line/column
3. **Missing Required Fields**: Continue parsing with warnings, use defaults
4. **Network Errors**: Propagate from EnvelopesApi with context
5. **No Files Matched**: Warn but don't error (consistent with send-event)

## 🧪 Testing Strategy

### Unit Tests

In `src/utils/apple_crash.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_crash() {
        let json = r#"{"exception": {"type": "EXC_BAD_ACCESS"}, ...}"#;
        let event = parse_ips_crash_report(json).unwrap();
        assert_eq!(event.platform, "cocoa");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        assert!(parse_ips_crash_report(json).is_err());
    }

    #[test]
    fn test_extract_threads() {
        // Test thread parsing logic
    }

    #[test]
    fn test_extract_debug_images() {
        // Test debug image conversion
    }
}
```

### Integration Tests

Test the full command flow with `.trycmd` files (see Step 6)

### Manual Testing

1. Get real `.ips` file from Xcode or App Store Connect
2. Upload dSYMs for that crash to Sentry project
3. Run: `sentry-cli send-apple-crash --release X.Y.Z path/to/crash.ips`
4. Verify in Sentry UI:
   - Event appears
   - Stack traces are symbolicated
   - Device/OS info is correct
   - Release association is correct

## 📦 Dependencies

All required dependencies are already in the project:

- `sentry` crate: Event structures and envelope handling
- `serde_json`: JSON parsing
- `anyhow`: Error handling
- `glob`: File pattern matching
- `clap`: CLI argument parsing

No new dependencies needed.

## 🚀 Rollout Plan

### Phase 1: Core Implementation
1. Implement parser module (Step 1)
2. Implement command module (Step 2)
3. Register command (Step 3)
4. Basic unit tests

### Phase 2: Testing
1. Create test fixtures (Step 5)
2. Create integration tests (Step 6)
3. Run `cargo test` and ensure all pass
4. Run `cargo fmt --all` to format code

### Phase 3: Validation
1. Manual testing with real .ips files
2. Verify symbolication works in Sentry
3. Test error cases
4. Test glob patterns

### Phase 4: Documentation
1. Update README.md with new command
2. Update CHANGELOG.md
3. Consider adding examples directory with sample commands

## ⚖️ Tradeoffs and Alternatives

### Alternative 1: Extend send-event instead of new command
**Pros**: Fewer commands, reuses existing code
**Cons**: Less discoverable, different file format from JSON events
**Decision**: New command (`send-apple-crash`) is clearer and more discoverable

### Alternative 2: Support local symbolication
**Pros**: Symbolicates before sending, works offline
**Cons**: Complex implementation, requires dSYM management, duplicates Sentry backend work
**Decision**: Let Sentry backend symbolicate (simpler, consistent)

### Alternative 3: Send as attachment instead of parsed event
**Pros**: Simpler implementation, preserves original file
**Cons**: May not symbolicate properly, less flexible for Sentry processing
**Decision**: Parse into Event structure (Approach A) as specified

### Alternative 4: Support both .ips and .crash formats initially
**Pros**: Broader compatibility
**Cons**: More complex parsing, .crash format is legacy
**Decision**: Start with .ips only, can add .crash later if needed

### Alternative 5: Require all metadata fields
**Pros**: Ensures complete data
**Cons**: Many .ips files may have minimal data
**Decision**: Make metadata optional, use sensible defaults

## 📊 Success Criteria

1. ✅ Command successfully parses valid .ips files
2. ✅ Events appear in Sentry UI
3. ✅ Stack traces are symbolicated when dSYMs available
4. ✅ Glob patterns work for batch uploads
5. ✅ Clear error messages for invalid inputs
6. ✅ All integration tests pass
7. ✅ Code follows existing sentry-cli patterns
8. ✅ `cargo fmt` and `cargo clippy` pass with no warnings

## 🔗 References

- Apple IPS Format: https://developer.apple.com/documentation/xcode/interpreting-the-json-format-of-a-crash-report
- GitHub Issue: https://github.com/getsentry/sentry-cli/issues/2663
- Sentry Event Payload: https://develop.sentry.dev/sdk/event-payloads/
- Sentry CLI Docs: https://docs.sentry.io/product/cli/

## 📝 Notes for Implementation

1. **Code Style**: Follow existing patterns in `send_event.rs` and `send_envelope.rs`
2. **Error Messages**: Make them actionable and user-friendly
3. **Logging**: Use `log::debug!` for verbose info, `log::warn!` for skipped data
4. **Platform**: Set `platform` to "cocoa" for proper Sentry processing
5. **SDK Info**: Use `get_sdk_info()` from `utils::event` to mark events as from sentry-cli
6. **Type Inference**: Prefer compiler inference, avoid explicit type annotations unless necessary
7. **Formatting**: Always run `cargo fmt --all` before committing

## 🎯 Final Checklist

Before considering the implementation complete:

- [ ] All code written and compiles without errors
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] `cargo fmt --all` run and committed
- [ ] `cargo clippy --workspace` passes with no warnings
- [ ] Manual testing completed with real .ips file
- [ ] Code follows Rust development guidelines from `.cursor/rules/`
- [ ] Commit message follows Sentry format: `feat(cli): Add send-apple-crash command for Apple crash reports`
- [ ] Error handling is comprehensive and user-friendly
- [ ] Documentation strings are clear and helpful
