# Deprecated Sentry CLI Commands and Options in JavaScript API

This document lists all deprecated Sentry CLI commands and options that are currently being used in the JavaScript API, along with their locations and recommended fixes.

## Deprecated Commands

### 1. `releases files upload-sourcemaps` (DEPRECATED)

**Location**: `js/releases/index.js:198`

**Current Usage**:
```javascript
const args = ['releases']
  .concat(helper.getProjectFlagsFromOptions(options))
  .concat(['files', release, 'upload-sourcemaps']);
```

**Deprecation Notice**: The entire `files` command group is deprecated and will be removed in a future version of `sentry-cli`. Use the `sourcemaps` command instead.

**How to Fix**: Replace the deprecated `files` command with the direct `sourcemaps upload` command:

```javascript
const args = ['sourcemaps', 'upload', release]
  .concat(helper.getProjectFlagsFromOptions(options));
```

**Impact**: This change affects the `uploadSourceMaps` method in the `Releases` class.

### 2. Test Files Using Deprecated Commands

**Location**: `js/__tests__/helper.test.js` (multiple lines: 50, 62, 85, 100, 141)

**Current Usage**: Test files are using the deprecated `releases files` command pattern:
```javascript
const command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];
```

**How to Fix**: Update all test cases to use the new `sourcemaps upload` command:
```javascript
const command = ['sourcemaps', 'upload', 'release', '/dev/null'];
```

## Deprecated Options

### 1. `--use-artifact-bundle` (DEPRECATED)

**Location**: `js/releases/options/uploadSourcemaps.js:57`

**Current Usage**:
```javascript
useArtifactBundle: {
  param: '--use-artifact-bundle',
  type: 'boolean',
},
```

**Deprecation Notice**: This option is deprecated and will be removed in the next major version. It was intended for internal use only.

**How to Fix**: Remove this option entirely from the schema. The functionality is no longer needed as artifact bundles are now the default behavior.

**Impact**: This affects the `uploadSourceMaps` method options schema.

### 2. `--rewrite` (DEPRECATED)

**Location**: `js/releases/options/uploadSourcemaps.js:18`

**Current Usage**:
```javascript
rewrite: {
  param: '--rewrite',
  invertedParam: '--no-rewrite',
  type: 'boolean',
},
```

**Deprecation Notice**: This flag has no effect and is left hidden for backward compatibility.

**How to Fix**: Remove this option from the schema as it no longer has any functionality.

**Impact**: This affects the `uploadSourceMaps` method options schema.

### 3. `--started` in Releases Finalize (DEPRECATED)

**Location**: `js/releases/options/deploys.js:8`

**Current Usage**:
```javascript
started: {
  param: '--started',
  type: 'number',
},
```

**Note**: This option is used in the deploys schema, but the deprecation warning is specifically for the `releases finalize` command. The deploys `--started` option appears to still be valid.

**Impact**: This option should be reviewed to ensure it's not being used inappropriately with the finalize command.

## Environment Variables

### 1. `SENTRY_FORCE_ARTIFACT_BUNDLES` (DEPRECATED)

**Location**: `src/commands/sourcemaps/upload.rs:433`

**Deprecation Notice**: This environment variable is deprecated and will be removed in the next major version.

**How to Fix**: Remove any usage of this environment variable in the JavaScript API configuration.

## Summary of Required Changes

1. **Update `uploadSourceMaps` method** in `js/releases/index.js` to use `sourcemaps upload` instead of `releases files upload-sourcemaps`
2. **Remove deprecated options** from `js/releases/options/uploadSourcemaps.js`:
   - `useArtifactBundle`
   - `rewrite`
3. **Update test files** in `js/__tests__/helper.test.js` to use the new command structure
4. **Review deploys options** to ensure `--started` is not being used inappropriately
5. **Remove any environment variable usage** of `SENTRY_FORCE_ARTIFACT_BUNDLES`

## Migration Timeline

These changes should be made before the next major version of `sentry-cli` where these deprecated features will be completely removed. The deprecation warnings are already being shown to users, so migration should be prioritized.

## Testing

After making these changes, ensure that:
1. All existing functionality continues to work
2. Source map uploads still function correctly
3. Test cases pass with the new command structure
4. No deprecation warnings are shown in the output