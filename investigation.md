# Investigation: Duplicate Source File Uploads in sentry-cli

## 🔍 Summary

**YES, duplicate uploads can occur**, but only in specific scenarios. The colleague's concern is valid for certain build configurations.

## 📋 Scenarios Analysis

### ✅ Scenario 1: Typical Modern Build (NO DUPLICATION)

**Setup:**
```
dist/
├── app.min.js          (minified bundle)
└── app.min.js.map      (sourcemap with sourcesContent)
```

**What happens:**
1. User runs: `sentry-cli sourcemaps upload dist/`
2. Files collected: `app.min.js` and `app.min.js.map` (both match default extensions)
3. Files uploaded: 2 files total
4. Original sources are embedded in `sourcesContent` field of the sourcemap
5. **Result: NO DUPLICATION** ✅

This is the most common scenario. Build tools (webpack, rollup, esbuild, etc.) typically output:
- Minified/bundled files in the output directory
- Sourcemaps with `sourcesContent` already populated
- Original source files remain in `src/` or similar directories (not uploaded)

### ⚠️ Scenario 2: Mixed Output Directory (DUPLICATION OCCURS)

**Setup:**
```
dist/
├── app.js              (original unbundled source)
├── app.min.js          (minified bundle)
└── app.min.js.map      (sourcemap referencing app.js, with sourcesContent)
```

**What happens:**
1. User runs: `sentry-cli sourcemaps upload dist/`
2. Files collected: ALL `.js` and `.map` files (per `DEFAULT_EXTENSIONS`)
   - `app.js` ✓
   - `app.min.js` ✓
   - `app.min.js.map` ✓
3. Files uploaded: 3 files total
4. **Result: DUPLICATION** ⚠️
   - `app.js` is uploaded as a standalone file
   - `app.js` is ALSO embedded in `app.min.js.map` via `sourcesContent`

### 🔍 Scenario 3: TypeScript/JSX Sources (NO DUPLICATION)

**Setup:**
```
dist/
├── app.tsx             (original TypeScript source)
├── app.min.js          (transpiled & minified)
└── app.min.js.map      (sourcemap with sourcesContent)
```

**What happens:**
1. User runs: `sentry-cli sourcemaps upload dist/`
2. Files collected: Only `app.min.js` and `app.min.js.map`
3. **Result: NO DUPLICATION** ✅
   - `app.tsx` is NOT uploaded (doesn't match `.js` or `.map` extensions)
   - Original TypeScript source is only in `sourcesContent`

## 🛠️ Technical Details

### Default File Extensions

From `src/commands/sourcemaps/upload.rs:22`:
```rust
const DEFAULT_EXTENSIONS: &[&str] = &["js", "cjs", "mjs", "map", "jsbundle", "bundle"];
```

Any file matching these extensions in the upload path will be collected and uploaded.

### The Rewrite Process

When `--no-rewrite` is NOT set (default behavior), `processor.rewrite()` is called with:

```rust
let options = sourcemap::RewriteOptions {
    load_local_source_contents: true,
    strip_prefixes: prefixes,
    ..Default::default()
};
```

The `load_local_source_contents: true` option tells the sourcemap library to:
1. Read the original source files from disk (if they exist)
2. Embed them into the sourcemap's `sourcesContent` field
3. This happens even if `sourcesContent` already exists

However, this does NOT prevent the original files from being uploaded if they were already collected.

### Upload Behavior

From the code flow in `src/utils/sourcemaps.rs` and `src/utils/file_upload.rs`:

1. **Collection phase**: All files matching extensions are added to `processor.sources`
2. **Rewrite phase**: Sourcemaps get their `sourcesContent` populated/updated
3. **Upload phase**: ALL files in `processor.sources` are uploaded

The validation logic in `validate_regular()` (line 1236) shows this clearly:
```rust
if sm.get_source_contents(idx).is_some() || source_urls.contains(source_url) {
    info!("validator found source ({source_url})");
}
```

A source is considered valid if it's EITHER in `sourcesContent` OR in the uploaded files. Both conditions can be true simultaneously.

## 📊 Evidence from Tests

From `tests/integration/_cases/sourcemaps/sourcemaps-upload-some-debugids.trycmd`:
- Input: `tests/integration/_fixtures/upload_some_debugids` (20 files)
- Output: "Bundled 20 files for upload"
- Breakdown:
  - 13 Scripts (.js files)
  - 7 Source Maps (.map files)

The sourcemap `server/chunks/1.js.map` contains:
- 163 sources in the `sources` field
- 163 entries in `sourcesContent` (all populated, not null)
- But the original source files (e.g., `node_modules/@sentry/core/build/cjs/api.js`) do NOT exist in the fixture directory

This demonstrates typical behavior where sourcemaps reference files via `webpack://` URLs that don't exist locally, so no duplication occurs.

## 🎯 When Does Duplication Occur?

Duplication happens when **all** of these conditions are met:

1. ✅ Original source files have `.js`, `.cjs`, or `.mjs` extensions
2. ✅ Original source files are in the same directory tree as the upload path
3. ✅ Sourcemaps already contain `sourcesContent` for these files
4. ✅ User uploads the entire directory without filtering

## 💡 Likelihood Assessment

**Low to Medium** likelihood in practice because:

- Modern build tools (webpack, vite, rollup, esbuild) typically output minified files separate from source files
- Most projects have `dist/` or `build/` folders containing only build outputs
- TypeScript/JSX sources don't match the default extensions
- Sourcemaps often reference sources via `webpack://` or similar URLs that don't exist as local files

However, it **can occur** if:
- Build configuration outputs both bundled and unbundled JS files to the same directory
- Developer explicitly copies source files to the dist folder
- Using a build setup that preserves original `.js` files alongside minified versions

## 🔧 Potential Solutions

If this is deemed a problem worth addressing:

### Option 1: Filter out files already in sourcesContent
Before uploading, check if a file is already embedded in any sourcemap's `sourcesContent` and skip uploading it as a standalone file.

**Pros:** Reduces upload size and duplication
**Cons:** Complex logic; might break edge cases where both are intentionally needed

### Option 2: Document expected usage patterns
Clarify in documentation that users should:
- Only upload their `dist/` or `build/` directories
- Not mix original sources with build outputs in the upload path
- Use `--ext` flag to be more selective if needed

**Pros:** Simple, no code changes needed
**Cons:** Relies on user behavior

### Option 3: Add a flag like `--skip-embedded-sources`
Allow users to opt into skipping files that are already embedded in sourcemaps.

**Pros:** Gives users control; backward compatible
**Cons:** Adds complexity; another flag to understand

## 📝 Conclusion

The colleague's observation is **correct**: sentry-cli can upload sources twice when both the original source files and sourcemaps (with sourcesContent) are in the upload path. However, this is **not the typical use case** due to how modern build tools organize their output.

The current behavior is technically correct (uploading what was requested) but could be optimized for the edge case where duplication occurs.

**Recommendation:** Document the expected usage pattern and potentially add a warning when duplication is detected, rather than changing the default behavior which might break existing workflows.
