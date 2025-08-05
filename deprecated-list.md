# Deprecated Sentry CLI Commands Used in JavaScript API

This document lists all identified deprecated Sentry CLI commands and options used in the Sentry CLI JavaScript API, along with their locations and migration instructions.

## Summary of Deprecated Commands

Based on analysis of the codebase and documentation, the following deprecated commands and patterns have been identified:

### 1. **Files Subcommands (DEPRECATED)**

**Location**: `/workspace/js/releases/index.js:198`

**Deprecated Command Pattern**:
```bash
sentry-cli releases files <release> upload-sourcemaps <path>
```

**Current Usage in JavaScript API**:
```javascript
// Located in: js/releases/index.js line 198
.concat(['files', release, 'upload-sourcemaps'])
```

**What it does**: Uploads source maps using the legacy files command structure.

**How to fix**: 
- **New Command**: Use `sentry-cli sourcemaps upload` instead
- **Migration**: Replace the entire command pattern:

```diff
// OLD (deprecated)
- ['releases', 'files', release, 'upload-sourcemaps', path]

// NEW (recommended)
+ ['sourcemaps', 'upload', '--release', release, path]
```

**Additional Context**: 
- Marked as deprecated in version 2.46.0 (changelog line 67)
- CLI shows deprecation warning: "⚠ DEPRECATION NOTICE: This functionality will be removed in a future version of `sentry-cli`. Use the `sourcemaps` command instead."
- The command is hidden in CLI help (`hide(true)`)

---

### 2. **useArtifactBundle Option (DEPRECATED)**

**Location**: `/workspace/js/releases/options/uploadSourcemaps.js:58-61`

**Deprecated Option**:
```javascript
useArtifactBundle: {
  param: '--use-artifact-bundle',
  type: 'boolean',
}
```

**What it does**: Forces the use of artifact bundles for source map uploads.

**How to fix**: 
- **Remove the option**: The `--use-artifact-bundle` flag is deprecated as of version 2.37.0
- **Migration**: Artifact bundles are now used by default, so simply remove this option from your configuration

```diff
// OLD (deprecated)
- useArtifactBundle: true

// NEW (automatic behavior)
+ // Remove this option entirely - artifact bundles are now default
```

---

### 3. **React Native AppCenter Command (DEPRECATED)**

**Location**: `/workspace/src/commands/react_native/appcenter.rs:24`

**Deprecated Command Pattern**:
```bash
sentry-cli react-native appcenter <args>
```

**What it does**: Uploads React Native projects for AppCenter (Microsoft App Center).

**How to fix**:
- **New Command**: Use `sentry-cli sourcemaps upload` instead
- **Migration**: Replace AppCenter-specific uploads with standard source map uploads

```diff
// OLD (deprecated)
- sentry-cli react-native appcenter --app-name MyApp --platform ios paths...

// NEW (recommended)  
+ sentry-cli sourcemaps upload --release <release> paths...
```

**Additional Context**: 
- Marked as deprecated in version 2.46.0 (changelog line 59)
- CLI shows deprecation warning
- Command is hidden in CLI help

---

### 4. **Send-Metric Commands (DEPRECATED)**

**Location**: `/workspace/src/commands/send_metric/mod.rs:16-20`

**Deprecated Commands**:
- `sentry-cli send-metric increment`
- `sentry-cli send-metric distribution` 
- `sentry-cli send-metric gauge`
- `sentry-cli send-metric set`

**What they do**: Send metric events to Sentry.

**How to fix**:
- **Migration**: These commands are being removed entirely
- **Deprecation Notice**: "Sentry will reject all metrics sent after October 7, 2024"
- **Alternative**: Use Sentry SDK metrics APIs instead of CLI commands

**Additional Context**:
- Marked as deprecated in version 2.37.0 (changelog line 424)
- Commands are hidden in CLI help
- Full removal planned for next major version

---

### 5. **Debug-Files --info-plist Argument (DEPRECATED)**

**Deprecated Command Pattern**:
```bash
sentry-cli debug-files upload --info-plist <path>
```

**What it does**: The `--info-plist` argument does nothing and is deprecated.

**How to fix**:
- **Remove the argument**: Simply remove `--info-plist` from your commands
- The argument has no functionality, so removal will not affect behavior

```diff
// OLD (deprecated)
- sentry-cli debug-files upload --info-plist Info.plist debug-files/

// NEW (correct)
+ sentry-cli debug-files upload debug-files/
```

**Additional Context**: 
- Explicitly deprecated in version 2.43.0 (changelog line 105-106)

---

## Test File Locations with Deprecated Usage

The following test files contain examples of deprecated command usage:

### JavaScript Tests
- `/workspace/js/__tests__/helper.test.js` (lines 50, 62, 85, 100, 141)
- `/workspace/js/releases/__tests__/index.test.js` (lines 58, 83, 108, 134, 150, 169)

These test files show the deprecated `releases files <release> upload-sourcemaps` pattern being used extensively.

## Migration Priority

1. **High Priority**: Files subcommands - actively used in the JavaScript API
2. **Medium Priority**: useArtifactBundle option - may be used in existing configurations  
3. **Low Priority**: React Native AppCenter - specific use case, likely limited usage
4. **Info Only**: Send-metric commands - not found in current JavaScript API
5. **Info Only**: Debug-files --info-plist - not found in current JavaScript API

## Recommendations

1. **Update the JavaScript API** to use the new `sourcemaps` command instead of the deprecated `files` command
2. **Remove useArtifactBundle option** from the options schema  
3. **Update tests** to use the new command patterns
4. **Update documentation** to reflect the new command structure
5. **Consider adding deprecation warnings** in the JavaScript API when deprecated patterns are used