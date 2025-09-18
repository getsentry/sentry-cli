## Node-test workflow failure investigation

- **Scope**: Release branch (all Node versions) – install step fails. Master passes.
- **Reproduction environment**: linux x64; Node v22.16.0; npm 10.9.2.

### Reproduction

Command executed in repo root:

```bash
npm ci --foreground-scripts
```

Observed failure (excerpt):

```
> @sentry/cli@2.54.0 postinstall
> node ./scripts/install.js

[sentry-cli] Sentry CLI failed to locate the "@sentry/cli-linux-x64" package after installation!
...
[sentry-cli] Downloading from https://downloads.sentry-cdn.com/sentry-cli/2.54.0/sentry-cli-Linux-x86_64

Error: Unable to download sentry-cli binary from https://downloads.sentry-cdn.com/sentry-cli/2.54.0/sentry-cli-Linux-x86_64.
Server returned: 403 - Forbidden
```

Exit code: 1

### Findings (evidence)

- The JS package version in this branch is pinned to `2.54.0`:
  - `package.json`:
    - `version: 2.54.0`
    - `optionalDependencies` include platform packages at `2.54.0` (e.g. `@sentry/cli-linux-x64: 2.54.0`).
- The expected platform package for this environment is `@sentry/cli-linux-x64@2.54.0`.
- Querying npm registry:

```bash
npm view @sentry/cli-linux-x64@2.54.0 version
# => 404 No match found for version 2.54.0

npm view @sentry/cli-linux-x64 versions --json | jq '.[-5:]'
# Last published: ... "2.53.0" (no 2.54.0)
```

- CDN artifact URL check:

```bash
curl -sI https://downloads.sentry-cdn.com/sentry-cli/2.54.0/sentry-cli-Linux-x86_64
# => HTTP/2 403
```

- Postinstall logic in `scripts/install.js` first resolves an optional dependency for the platform; when missing, it falls back to downloading from the CDN. Both paths fail here because:
  - The platform package `@sentry/cli-linux-x64@2.54.0` is not published on npm.
  - The CDN binary for 2.54.0 responds 403.

### Root cause

The release branch references a version `2.54.0` whose platform-specific binary distributions are not available:
- Platform npm packages at `2.54.0` are missing.
- Corresponding CDN artifacts for `2.54.0` return 403.

As a result, `npm ci` fails during `postinstall` of `@sentry/cli` because neither the optional dependency install nor the CDN fallback can supply the binary.

This explains why master passes (likely points to an available/published version) while this release branch fails (points to an unpublished/unavailable version).

### Fix options

- Update the version references in the release branch to a version whose platform packages and CDN artifacts exist (e.g. latest published series, such as `2.53.0` at the time of this investigation), keeping `optionalDependencies` in sync.
- Alternatively, publish the missing platform packages (`@sentry/cli-<platform>@2.54.0`) and ensure the `2.54.0` binaries are available on the CDN, then re-run the workflow.
- Temporary CI-only workaround (not recommended for releases): set `SENTRYCLI_SKIP_DOWNLOAD=1` and provide `SENTRY_BINARY_PATH` to a known-good binary, bypassing network download. This should only be used if binary provenance and licensing are handled.

### Recommendation

Prefer aligning the release branch to the latest fully published version (npm + CDN). If `2.54.0` is the intended release, publish all platform packages and make the CDN artifacts available before retrying CI.