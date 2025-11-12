# Changelog

## Unreleased

### Improvements

- Added validation for the `sentry-cli build upload` command's `--head-sha` and `--base-sha` arguments ([#2945](https://github.com/getsentry/sentry-cli/pull/2945)). The CLI now validates that these are valid SHA1 sums. Passing an empty string is also allowed; this prevents the default values from being used, causing the values to instead be unset.

### Fixes

- Fixed a bug where providing empty-string values for the `sentry-cli build upload` command's `--vcs-provider`, `--head-repo-name`, `--head-ref`, `--base-ref`, and `--base-repo-name` arguments resulted in 400 errors ([#2946](https://github.com/getsentry/sentry-cli/pull/2946)). Now, setting these to empty strings instead explicitly clears the default value we would set otherwise, as expected.

## 2.58.1

### Deprecations

- Deprecated API key authentication ([#2934](https://github.com/getsentry/sentry-cli/pull/2934), [#2937](https://github.com/getsentry/sentry-cli/pull/2937)). Users who are still using API keys to authenticate Sentry CLI should generate and use an [Auth Token](https://docs.sentry.io/account/auth-tokens/) instead.

### Improvements

- The `sentry-cli debug-files bundle-jvm` no longer makes any HTTP requests to Sentry, meaning auth tokens are no longer needed, and the command can be run offline ([#2926](https://github.com/getsentry/sentry-cli/pull/2926)).

### Fixes

- Skip setting `base_sha` and `base_ref` when they equal `head_sha` during auto-inference, since comparing a commit to itself provides no meaningful baseline ([#2924](https://github.com/getsentry/sentry-cli/pull/2924)).
- Improved error message when supplying a non-existent organization to `sentry-cli sourcemaps upload`. The error now correctly indicates the organization doesn't exist, rather than incorrectly suggesting the Sentry server lacks artifact bundle support ([#2931](https://github.com/getsentry/sentry-cli/pull/2931)).

## 2.58.0

### New Features

- Removed experimental status from the `sentry-cli build upload` commands ([#2899](https://github.com/getsentry/sentry-cli/pull/2899), [#2905](https://github.com/getsentry/sentry-cli/pull/2905)). At the time of this release, build uploads are still in closed beta on the server side, so most customers cannot use this functionality quite yet.
- Added CLI version metadata to build upload archives ([#2890](https://github.com/getsentry/sentry-cli/pull/2890)).

### Deprecations

- Deprecated the `upload-proguard` subcommand's `--platform` flag ([#2863](https://github.com/getsentry/sentry-cli/pull/2863)). This flag was a no-op for some time, so we will remove it in the next major.
- Deprecated the `upload-proguard` subcommand's `--android-manifest` flag ([#2891](https://github.com/getsentry/sentry-cli/pull/2891)). This flag was a no-op for some time, so we will remove it in the next major.
- Deprecated the `sentry-cli sourcemaps upload` command's `--no-dedupe` flag ([#2913](https://github.com/getsentry/sentry-cli/pull/2913)). The flag was no longer relevant for sourcemap uploads to modern Sentry servers and was made a no-op.

### Fixes

- Fixed autofilled git base metadata (`--base-ref`, `--base-sha`) when using the `build upload` subcommand in git repos. Previously this worked only in the context of GitHub workflows ([#2897](https://github.com/getsentry/sentry-cli/pull/2897), [#2898](https://github.com/getsentry/sentry-cli/pull/2898)).

### Performance

- Slightly sped up the `sentry-cli sourcemaps upload` command by eliminating an HTTP request to the Sentry server, which was not required in most cases ([#2913](https://github.com/getsentry/sentry-cli/pull/2913)).

## 2.57.0

### New Features

- (JS API) Add `projects` field to `SentryCliUploadSourceMapsOptions` ([#2856](https://github.com/getsentry/sentry-cli/pull/2856))

### Deprecations

- Deprecated the `upload-proguard` subcommand's `--app-id`, `--version`, and `--version-code` flags ([#2852](https://github.com/getsentry/sentry-cli/pull/2852)), as we plan to remove these flags in Sentry CLI 3.x. Users should simply stop using the flags; the values specified there have never had an effect on deobfuscation, and are no longer visible in Sentry.
- Added a deprecation notice for release bundle uploads, a legacy method for uploading source maps ([#2844](https://github.com/getsentry/sentry-cli/pull/2844)). Release bundle uploads will be removed in Sentry CLI 3.x in favor of artifact bundles, the newer source map upload method [introduced in Sentry version 23.6.2](https://github.com/getsentry/sentry/commit/f90f764fda09575f3f94caf32d04589098384616). **Self-hosted users**: You must upgrade to Sentry 23.6.2 or later before upgrading to Sentry CLI 3.x.

### Fixes

- Fixed a bug where some log messages would not show up in CI environments or when redirecting stderr to a file ([#2830](https://github.com/getsentry/sentry-cli/pull/2830)). Specifically, this bug was affecting any subcommand that uses a progress bar, such as `sentry-cli debug-files bundle-jvm` and `sentry-cli sourcemaps upload`. Any stderr output during the progress bar was lost if stderr was redirected.

## 2.56.1

### Deprecations

- Added a deprecation notice for legacy uploading methods ([#2836](https://github.com/getsentry/sentry-cli/pull/2836), [#2837](https://github.com/getsentry/sentry-cli/pull/2837))
  - Support for these legacy uploading methods, required to upload to self-hosted Sentry servers below version 10.0.0, will be removed in the next major release (3.x). If you observe these new deprecation notices, we recommend upgrading your self-hosted Sentry server, or pinning Sentry CLI to a compatible version (2.x).
  - You may encounter these deprecation notices when uploading debug files or sourcemaps.

### Fixes & improvements

- Fixed a bug with sourcemap injection ([#2764](https://github.com/getsentry/sentry-cli/pull/2764)) by @szokeasaurusrex
  - This change ensures we do not attempt to associate multiple compiled sources with the same sourcemap. As there should be at most one sourcemap for each compiled source, associating multiple compiled sources with the same sourcemap would lead to an invalid state.
- Updated some outdated dependencies ([#2816](https://github.com/getsentry/sentry-cli/pull/2816), [#2818](https://github.com/getsentry/sentry-cli/pull/2818), and [#2819](https://github.com/getsentry/sentry-cli/pull/2819))

## 2.56.0

### Various fixes & improvements

- feat: auto-fetch head-ref from GitHub Actions in detached HEAD state (#2805) by @runningcode
- feat: automatically fetch base SHA in GitHub Actions PR workflows (#2799) by @runningcode
- feat(preprod): use deflated compression when creating the zip file (#2800) by @trevor-e
- feat(preprod): make sure at least one app bundle is present for upload (#2795) by @trevor-e
- feat(preprod): fail upload if app is missing Info.plist (#2793) by @trevor-e
- feat: restore GitHub Actions base branch detection (#2792) by @runningcode
- fix: lower log level for missing base ref detection (EME-369) (#2813) by @runningcode
- fix: simplify debug logging for PR number detection (EME-362) (#2812) by @runningcode
- fix: serialize VCS tests to prevent race conditions (EME-368) (#2811) by @runningcode
- fix: Validate `SENTRY_RELEASE` environment variable (#2807) by @szokeasaurusrex
- fix: use actual PR head SHA in GitHub Actions instead of merge commit (#2785) by @runningcode
- fix: suppress warning messages in failing build upload tests (#2791) by @runningcode

## 2.55.0

### Various fixes & improvements

- feat(build): preserve repository name case for build upload (#2777) by @runningcode
- fix(sourcemaps): Display injection errors (#2775) by @szokeasaurusrex
- feat: Normalize VCS provider names to match backend (#2770) by @runningcode
- feat: Improve upload error message to show cause (#2765) by @runningcode
- fix: Safer asset catalog reader for liquid glass (#2771) by @noahsmartin
- fix(releases): handle partial SHAs correctly in commit resolution (#2734) by @srest2021

## 2.54.0

### Various fixes & improvements

- Fix: symlinks in normalized upload (#2744) by @noahsmartin
- feat(vcs): Prefer upstream remote over origin for base repo name (#2737) by @runningcode
- feat(build): Add auto-detection of base_repo_name from git remote (#2735) by @runningcode
- feat(build): Add auto-detection of PR number from GitHub Actions (#2722) by @runningcode
- feat(build): Auto-detect base_ref from git merge-base (#2720) by @runningcode
- feat(logs): support log streaming (#2666) by @vgrozdanic

## 2.53.0

### Various fixes & improvements

- feat(mobile-app): Add release notes option (#2712) by @noahsmartin

### Changes from 2.53.0-alpha

2.53.0-alpha reintroduced the `build` (previously named `mobile-app`) commands. 2.53.0 is the first stable release to reintroduce them.

Please note, the `build` commands are still experimental, and are therefore subject to breaking changes, including removal, in any release, without notice.

- feat(mobile-app): Add default vcs base_ref parsing for mobile-app subcommand (#2706) by @rbro112
- chore(mobile-app): Rename mobile-app subcommand to build (#2719) by @rbro112
- Revert "feat(mobile-app): Reintroduce mobile-app feature gating (#2643)" (#2670) by @noahsmartin
- meta(cursor): Add rule to avoid explicit type annotations (#2717) by @szokeasaurusrex
- retry on cloudflare timeout (#2695) by @manishrawat1992

## 2.53.0-alpha

This release reintroduces the `build` (previously named `mobile-app`) commands.

### Various fixes & improvements

- feat(mobile-app): Add default vcs base_ref parsing for mobile-app subcommand (#2706) by @rbro112
- chore(mobile-app): Rename mobile-app subcommand to build (#2719) by @rbro112
- Revert "feat(mobile-app): Reintroduce mobile-app feature gating (#2643)" (#2670) by @noahsmartin
- meta(cursor): Add rule to avoid explicit type annotations (#2717) by @szokeasaurusrex
- retry on cloudflare timeout (#2695) by @manishrawat1992

## 2.52.0

### Various fixes & improvements

- feat(dart): add `dart-symbol-map upload` command (#2691) by @buenaflor
- Add default vcs head_repo_name and provider parsing for mobile-app subcommand (#2699) by @rbro112
- ref(debug-files): Remove unnecessary `collect` (#2705) by @szokeasaurusrex
- build(deps): bump github/codeql-action from 3.29.5 to 3.29.8 (#2700) by @dependabot
- build(deps): bump actions/checkout from 4.2.2 to 5.0.0 (#2701) by @dependabot
- build(deps): bump actions/create-github-app-token from 2.0.6 to 2.1.1 (#2702) by @dependabot
- build(deps): bump actions/download-artifact from 4.3.0 to 5.0.0 (#2703) by @dependabot
- Use URL from backend (#2698) by @chromy
- feat(mobile-app): Add new VCS params to mobile-app command (#2682) by @rbro112
- feat(launchpad): Add asset catalog files to zip without adding to folder (#2667) by @noahsmartin
- feat(preprod): Show analysis URL after mobile-app upload (#2675) by @chromy
- feat(logs): allow project slugs in logs list (#2688) by @shellmayr
- fix(debug-files): Run all processing steps with `--no-upload` (#2693) by @P1n3appl3

## 2.51.1

### Various fixes & improvements

- fix(logs): Mark `logs` command as beta (#2690) by @szokeasaurusrex

## 2.51.0

### Logs command

This release introduces a [new `sentry-cli logs list` command](https://github.com/getsentry/sentry-cli/pull/2664), which lists your Sentry logs. For more details, see `sentry-cli logs list --help`.

Please note: if you receive a `403` error when using the new command, you may need to generate a new auth token with different permissions. You can do this by navigating to _Settings_ → _Developer Settings_ → _Personal Tokens_ in Sentry. On the _Personal Tokens_ page, generate a token with `org:read` scope, and pass this to the command using `--auth-token` or by setting the `SENTRY_AUTH_TOKEN` environment variable.

### Various fixes & improvements

- fix(js): Stop calling `releases files upload-sourcemaps` (#2676) by @szokeasaurusrex

## 2.50.2

This change removes the `mobile-app upload` experimental feature which was introduced in 2.50.1.

## 2.50.1

### Various fixes & improvements

- build(deps): bump form-data from 3.0.1 to 3.0.4 (#2638) by @dependabot

### New experimental feature

This release introduces the new experimental `mobile-app upload` subcommand. This command is experimental, and therefore subject to changes, including breaking changes and/or removal, in any release. The command might not work as expected.

## 2.50.0

### Various fixes & improvements

- feat(js): Expose `rejectOnError` in public `execute` type declarations (#2628) by @Lms24

## 2.49.0

### Various fixes & improvements

- feat(js): Export `live` option type for `releases.uploadSourceMaps` (#2627) by @Lms24

## 2.48.0

### Various fixes & improvements

- feat(js): Add `live: 'rejectOnError'` execution mode to `execute` method (#2605) by @Lms24

### Build-time changes

- feat: allow optionally disabling Swift sandbox (#2587) by @carlocab

## 2.47.1

No user-facing changes.

## 2.47.0

### Various fixes & improvements

- ref: Exclude `mobile-app` command from release builds (#2582) by @szokeasaurusrex
- feat(login): Improve error output for login errors (#2581) by @szokeasaurusrex
- test(monitors): pass empty options to grep (#2562) by @lcian
- feat(login): Warn when overwriting existing auth token (#2554) by @szokeasaurusrex
- meta: Add .sentryclirc to gitignore (#2547) by @rbro112
- build: Bump MSRV to 1.86

## 2.46.0

### Various fixes & improvements

- feat: Mark `react-native appcenter` as deprecated (#2522) by @chromy
- docs: Fix typo "the the" -> "the" (#2519) by @chromy
- feat(npm): Add support for installing `npm` package on Android (#2524) by @szokeasaurusrex
- feat: Retry all HTTP requests (#2523) by @szokeasaurusrex
- ref: Only obtain max retry count once (#2521) by @szokeasaurusrex
- fix: Don't error if invalid value supplied for max retries (#2520) by @szokeasaurusrex
- fix: Explicitly deprecate `--started` flag (#2515) by @szokeasaurusrex
- fix: Use `orig_path` for bundle sources output file name (#2514) by @szokeasaurusrex
- feat: Mark all `files` subcommands as deprecated. (#2512) by @Swatinem
- Support multiple files in SENTRY_DOTENV_PATH (#2454) by @Kinqdos
- fix(sourcemaps): Avoid associating only sourcemap with all minified sources (#2447) by @szokeasaurusrex

## 2.45.0

### New feature

- feat(sourcemaps): Multi-project sourcemaps upload (#2497) by @szokeasaurusrex
  - Sourcemaps can now be uploaded to multiple projects at once by passing each project to the `sentry-cli sourcemaps upload` command, like so:
    ```sh
    sentry-cli sourcemaps upload -p project1 -p project2 /path/to/sourcemaps
    ```
  - Note that users on old versions of self-hosted Sentry may need to upgrade their self-hosted server to a newer version to take advantage of multi-project uploads.

### Various fixes & improvements

- ref: Rename `fixup_js_file_end` (#2475) by @szokeasaurusrex
- ref: Use slice instead of vec for assemble artifact request (#2498) by @szokeasaurusrex
- ref: Separate `LegacyUploadContext` for legacy uploads (#2494) by @szokeasaurusrex
- feat: Remove organization and project info log (#2490) by @szokeasaurusrex

## 2.44.0

### Various fixes & improvements

- feat(sourcemaps): Support injecting indexed sourcemaps (#2470) by @szokeasaurusrex
- test(sourcemaps): Assert injection outputs (#2483) by @szokeasaurusrex

## 2.43.1

### Various fixes & improvements

- build: Bump `tokio` to `1.44.2` (#2474) by @szokeasaurusrex
- chore: Fix nightly clippy lints (#2469) by @loewenheim

## 2.43.0

This release explicitly deprecates the `sentry-cli debug-files upload` command's `--info-plist` argument, since the argument does nothing. If you are using the `--info-plist` argument, you should stop using it.

### Various fixes & improvements

- fix(debug_files): Don't call `xcodebuild` (#2443) by @szokeasaurusrex

## 2.42.5

This is a re-release of 2.45.5-alpha as a stable release. There are no code changes from 2.45.5-alpha, since the Windows ARM build for that version was published successfully.

## 2.42.5-alpha

This release adds a binary for Windows on ARM.

We are releasing this as an alpha to test that the new Windows ARM build is published to NPM correctly.

### Various fixes & improvements

- ci(npm): Release Windows ARM build to `npm` (#2436) by @szokeasaurusrex
- ci: Run lint action on all target operating systems (#2360) by @szokeasaurusrex
- ref: Simplify `is_homebrew_install` (#2434) by @szokeasaurusrex
- build(deps): bump github/codeql-action from 3.28.12 to 3.28.13 (#2435) by @dependabot
- ref: Simplify `set_executable_mode` (#2433) by @szokeasaurusrex
- ci: Build for Windows ARM (#2429) by @szokeasaurusrex
- build: Replace `username` with `whoami` (#2432) by @szokeasaurusrex
- build: Remove direct `winapi` dependency (#2431) by @szokeasaurusrex
- build(deps): bump actions/create-github-app-token from 1.11.0 to 1.11.7 (#2430) by @dependabot
- ci: Auto-update GHA with Dependabot (#2428) by @szokeasaurusrex
- ci: Update and securely pin all actions (#2427) by @szokeasaurusrex
- ci: Remove unneeded `-D warnings` (#2425) by @szokeasaurusrex
- ci: Stop using `actions-rs` (#2424) by @szokeasaurusrex
- deps: Update zip to 2.4.2 (#2423) by @loewenheim
- build: update `zip` dependency (#2421) by @szokeasaurusrex

## 2.42.4

### Various fixes & improvements

- build(macos): Sign macOS binaries (#2401) by @szokeasaurusrex
- ci(docker): Fix GHCR releases so they are multiarch (#2418) by @szokeasaurusrex

## 2.42.3

### Various fixes & improvements

- fix(sourcemaps): Fix mismatches between path and URL on Windows (#2414) by @BYK

## 2.42.2

### Various fixes & improvements

- fix(sourcemaps): Exclude skipped files from bundle file count (#2397) by @a-hariti
- fix: warn about missing SENTRY_RELEASE when it's set to empty string (#2394) by @a-hariti
- build(node): Bump `semver@5.*` dependencies (#2399) by @szokeasaurusrex
- build(node): Bump `semver@6.*` (#2400) by @szokeasaurusrex
- build(node): Bump `semver@^7.*` dependencies (#2398) by @szokeasaurusrex
- build(node): Bump `cross-spawn@7.0.X` (#2396) by @szokeasaurusrex
- build(node): Bump `cross-spawn@^6.0.5` (#2395) by @szokeasaurusrex
- ci(docker): Release Docker image to GHCR (#2393) by @szokeasaurusrex
- ci(docker): Fix caching (#2391) by @szokeasaurusrex

## 2.42.1

This release is a re-release of 2.42.0. There are no code changes to the CLI. We are performing this re-release because 2.42.0 failed to publish to Docker Hub.

### Various fixes & improvements

- ci(docker): Don't publish to GHCR via Craft (#2392) by @szokeasaurusrex

## 2.42.0

With this release, we now build and publish multi-architecture (arm64 and aarch64) Docker images for Sentry CLI.

### Various fixes & improvements

- feat: Only warn for token org mismatch (#2382) by @szokeasaurusrex
- feat: Improve custom panic hook (#2355) by @szokeasaurusrex
  - We now have better error messages when there is an internal error in Sentry CLI.
- feat: Deprecate `--use-artifact-bundle` option (#2349) by @szokeasaurusrex
- feat: Deprecate `useArtifactBundle` JS option (#2348) by @szokeasaurusrex
- fix(update): Properly handle errors when getting latest CLI version (#2370) by @szokeasaurusrex

<details>
<summary><h3>Non-user-facing changes</h3></summary>

- ci(docker): Add Craft targets for `docker` (#2390) by @szokeasaurusrex
- ci(docker): Enable caching of Docker builds (#2389) by @szokeasaurusrex
- ci(docker): Remove invalid argument (#2388) by @szokeasaurusrex
- ci(docker): Build each architecture natively (#2387) by @szokeasaurusrex
- build(docker): Build Docker image on host architecture (#2386) by @szokeasaurusrex
- build: Use hash instead of branch in GHCR tag (#2384) by @szokeasaurusrex
- build: Build Docker image in CI and publish to GHCR (#2383) by @szokeasaurusrex
- ci: Pin Actions runners' OS versions (#2353) by @szokeasaurusrex
- meta: Update LICENSE (#2380) by @szokeasaurusrex
- build: Update `url` crate (#2379) by @szokeasaurusrex
- docs: Explain why lint is disabled (#2371) by @szokeasaurusrex
- ref(sourcemaps): Fix `unnecessary_wraps` for `add_debug_id_references` (#2369) by @szokeasaurusrex
- ref(sourcemaps): Fix `unnecessary_wraps` for `add_sourcemap_references` (#2368) by @szokeasaurusrex
- ref(sourcemaps): Fix `unnecessary_wraps` lint for `SourceMapsProcessor::add` (#2367) by @szokeasaurusrex
- ref(sourcemaps): Make `lookup_pos` not return `Option` (#2366) by @szokeasaurusrex
- ref: Make `Api::with_body` return `Self` (#2363) by @szokeasaurusrex
- ref(api): Make `with_retry` return `Self` (#2365) by @szokeasaurusrex
- ref(api): Make `progress_bar_mode` return `Self` (#2364) by @szokeasaurusrex
- ref(config): Fix `unnecessary_wraps` lint for `set_auth` (#2362) by @szokeasaurusrex
- ref(config): Remove unneeded `Result` from `Config::from_file` (#2361) by @szokeasaurusrex
- ref: Enable `clippy::unnecessary_wraps` lint (#2358) by @szokeasaurusrex
- ci: Change lint action to `-D warnings` (#2359) by @szokeasaurusrex
- ref: Simplify `--log-level` parsing (#2356) by @szokeasaurusrex
- docs: Correct typo in doc string (#2354) by @szokeasaurusrex

</details>

## 2.41.1

### Various fixes & improvements

- build: Replace `dotenv` with `dotenvy` (#2351) by @szokeasaurusrex
  - This fixes a problem where multiline env variables were not supported in `.env` files

## 2.41.0

### Various fixes & improvements

- build: Bump `symbolic` to `12.13.3` (#2346) by @szokeasaurusrex
- ref(api): Replace custom deserializer with derive (#2337) by @szokeasaurusrex
- ref(sourcemaps): Reduce sourcemap upload memory usage (#2343) by @szokeasaurusrex
- build: Update `memmap2` (#2340) by @szokeasaurusrex
- ref: Fix new clippy lints (#2341) by @szokeasaurusrex
- feat(dif): Fail `debug-files upload` when file is too big (#2331) by @szokeasaurusrex
- ref(dif): Handle "too big" error with warning (#2330) by @szokeasaurusrex
- ref(dif): Create type for DIF validation errors (#2329) by @szokeasaurusrex
- ref(api): Remove unnecessary `collect` (#2333) by @szokeasaurusrex

## 2.40.0

### New features

- feat(debugid): Update debug ID snippet to write on `globalThis` when available (#2301) by @lforst

### Improvements

- build: Change release opt-level (#2325) by @szokeasaurusrex
- build: Make backtraces useable in release builds (#2324) by @szokeasaurusrex

### Bug fixes

- fix(chunking): Remove power-of-two chunk size restriction (#2312) by @szokeasaurusrex

<details>
<summary><h3>Non-user-facing changes</h3></summary>

We made several refactors and added several tests in this release. These changes should not affect users.

- ref(sourcemaps): Reword "no sourcemap ref" (#2320) by @szokeasaurusrex
- test(proguard): Add chunk upload tests (#2322) by @szokeasaurusrex
- ref(proguard): Use existing chunked upload logic (#2318) by @szokeasaurusrex
- ref(chunks): Remove `upload-dif` reference from `poll_assemble` (#2321) by @szokeasaurusrex
- ref(chunks): Make `render_detail` take `Option<&str>` (#2317) by @szokeasaurusrex
- ref(chunks): Extract upload logic from `upload_difs_chunked` (#2316) by @szokeasaurusrex
- ref(chunks): Rename `upload` module to `options` (#2315) by @szokeasaurusrex
- ref(chunks): Make `ChunkOptions` a `struct` (#2314) by @szokeasaurusrex
- ref(dif): Use `&str` in `DifUpload` struct (#2307) by @szokeasaurusrex
- ref(dif): Genericize `poll_assemble` (#2300) by @szokeasaurusrex
- feat(release): Replace release bot with GH app (#2306) by @Jeffreyhung
- fix(proguard): Add leading `/` to uploaded Proguard file name (#2304) by @szokeasaurusrex
- ref(dif): Genericize `try_assemble` `options` parameter (#2299) by @szokeasaurusrex
- ref(api): Rename `ChunkedUploadOptions` to indicate they are set by server (#2298) by @szokeasaurusrex
- ref(proguard): Use `Chunked<ProguardMapping>` for proguard upload (#2296) by @szokeasaurusrex
- ref(chunks): Make `ChunkedDifRequest` take `Cow<'_, str>` for `name` (#2295) by @szokeasaurusrex
- ref(proguard): Replace `MappingRef` with `ProguardMapping` (#2294) by @szokeasaurusrex
- ref(proguard): Create new `proguard` `utils` submodule (#2293) by @szokeasaurusrex
- ref(proguard): Directly open paths as `ByteView` (#2292) by @szokeasaurusrex
- ref(dif): Put hash in `ChunkedDifRequest` (#2290) by @szokeasaurusrex
- ref(chunks): Rename `Named` trait to `Assemblable` (#2289) by @szokeasaurusrex
- ref(dif): Make `poll_assemble` generic (#2287) by @szokeasaurusrex
- ref(dif): Rename `ToAssemble` trait
- ref(dif): Make `try_assemble_dif` generic
- ref(dif): Replace `ChunkedDifMatch` with generic `ChunkedObject`
- ref(utils): Use `usize` in `get_sha1_checksums` signature
- test(chunk-upload): Test chunk uploading only some chunks missing
- ref: Fix new Clippy lints
- test(chunk-upload): Test upload where only some files need upload (#2276)
- test(chunk-upload): Test chunk upload with many chunks (#2275)
- ref(test): Use constant for expected request boundary (#2277)
- test(chunk-upload): Add a test for uploading multiple debug files (#2274)
- ref(sourcemaps): Fix clippy lint
- ref(test): Introduce `assert_cmd` test manager
- test(chunk-upload): Add test for full chunk upload

</details>

## 2.39.1

### Various fixes & improvements

- fix(sourcemaps): Correctly read files with debug_id and debugId (#2268) by @loewenheim
- build: Remove unused dependencies (#2255) by @szokeasaurusrex
- ref(proguard): Define environment variable name in constant (#2265) by @szokeasaurusrex
- ref(test): Remove redundant line (#2266) by @szokeasaurusrex
- fix(proguard): Wait until chunks are assembled (#2267) by @szokeasaurusrex

## 2.39.0

### Features/improvements

- feat(proguard): Introduce experimental chunk uploading feature (#2264) by @szokeasaurusrex
- feat: Read debug IDs from `debugId` field in sourcemaps (#2222)

### Various fixes & improvements

- ref(proguard): Delete pointless code (#2263) by @szokeasaurusrex
- fix: Improve error handling in dif.rs (#2225)
- ci: Run codeql-analysis on all PRs (#2224)
- build: Add `assert_cmd` as a dev dependency
- ref(api): Remove dead code (#2217)
- ref: Remove `SENTRY_DUMP_REPONSE` environment variable (#2212)
- ref(utils): Remove `allow(dead_code)` from update utils (#2216)
- ref(api): Remove blanket `allow(dead_code)` (#2215)
- ref(api): Only allow dead code where needed (#2213)

<details>
<summary><h3>Changes to tests</h3></summary>
  
- ref(test): Broaden `with_header_matcher` types (#2261) by @szokeasaurusrex
- ref(test): Accept `impl Into<Matcher>` for `with_matcher` (#2260) by @szokeasaurusrex
- ref(test): Align `with_reponse_body` parameter to `mockito` (#2259) by @szokeasaurusrex
- ref(test): Make mock builder status default to `200` (#2258) by @szokeasaurusrex
- ref(test): Change chunk upload options org (#2257) by @szokeasaurusrex
- ref(test): Bump `mockito` mocking library (#2254) by @szokeasaurusrex
- ref(test): Use `TestManager` in new chunk upload test (#2253) by @szokeasaurusrex
- ref(test): Add `TestManager` struct for uniform test setup (#2252) by @szokeasaurusrex
- ref(tests): `mock_common_endpoints` refactor (#2251) by @szokeasaurusrex
- ref(tests): Simplify `mock_common_upload_endpoints` (#2250) by @szokeasaurusrex
- ref(tests): Extract `mock_common_upload_endpoints` into module (#2249) by @szokeasaurusrex
- ref(tests): Rename `mocking` to `mock_endpoint_builder` (#2248) by @szokeasaurusrex
- ref: Fix typo in tests (#2243) by @szokeasaurusrex
- ref(test): Rename `EndpointOptions` to `MockEndpointBuilder` (#2247) by @szokeasaurusrex
- ref(test): Refactor `EndpointOptions` (#2246) by @szokeasaurusrex
- ref(test): Privatize struct fields of `EndpointOptions` (#2245) by @szokeasaurusrex
- ref(test): Move mock endpoint code to module (#2244) by @szokeasaurusrex
- test: batch send-event tests (#2230) by @szokeasaurusrex
- test: Run `trycmd` tests with `RUST_BACKTRACE=0` (#2242) by @szokeasaurusrex
- test(send-metric): Batch tests together (#2241) by @szokeasaurusrex
- test(react_native): Bubble up `target_os` (#2240) by @szokeasaurusrex
- test(projects): Batch tests together (#2239) by @szokeasaurusrex
- test(monitors): Batch monitors tests (#2236)
- test(monitors): Delete unneeded monitors test (#2237)
- test(organizations): Batch tests together (#2238)
- test(events): Batch tests together (#2235)
- test(debug-files): Batch tests together (#2234)
- test: batch token validation tests (#2231)
- test: batch upload-proguard tests (#2233)
- test: batch update tests (#2232)
- test: Batch org token tests together (#2229)
- test: Batch `bash_hook` tests together (#2226)
- test: batch login tests (#2228)
- test: Batch send envelope tests together (#2227)
- test(debug-files): Add test for `assemble` endpoint call
- ref(tests): Decouple test env vars from trycmd code

</details>

## 2.38.2

### Various fixes & improvements

- deps: Update symbolic to 12.12.0 (#2210) by @loewenheim

## 2.38.1

### Various fixes & improvements

- meta: Remove `.vscode` directory from version control (#2208) by @szokeasaurusrex
- build(windows): Statically link CRT on Windows (#2203) by @szokeasaurusrex
- ref: Update `main` to indicate it does not return (#2192) by @szokeasaurusrex
- ref: Indicate that `commands::main` does not return (#2191) by @szokeasaurusrex

## 2.38.0

### Various fixes & improvements

- feat(errors): Print backtrace when `RUST_BACKTRACE=1` (#2189) by @szokeasaurusrex
- fix(crons): Don't panic when passing `--auth-token` (#2172) by @szokeasaurusrex

## 2.37.0

All Xcode uploads are now executed in the foreground, which should allow for easier debugging of any problems that occur while uploading files during the build process, since errors will be logged directly within Xcode.

With this change, the `--force-foreground` flag is no longer needed, since we always upload in the foreground. The flag is now a deprecated no-op.

## 2.36.6

There are no code changes in this release. It is being performed to test an update to the release build process.

## 2.36.5

There are no code changes in this release. It is a re-release of 2.36.4, which we are making because 2.36.4 and 2.36.3
failed to publish to PyPI.

## 2.36.4

This releases fixes includes a bugfix (#2171 by @szokeasaurusrex) for #2169. The bug caused any command run with
`sentry-cli monitors run` to not be executed whenever sending the cron checkin to Sentry failed, e.g. during a Sentry
outage or due to some other network failure. With the bugfix, we log the error and execute the program even when there
was an error sending the checkin.

**We recommend that all users using `sentry-cli monitors run` upgrade to Sentry CLI version 2.36.4 immediately.**

## 2.36.3

### Various fixes & improvements

- build: Upgrade `curl-sys` (#2164) by @szokeasaurusrex

## 2.36.2

### Various fixes & improvements

- fix(deploys): Honor `--project` in `deploys new` subcommand (#2160) by @szokeasaurusrex
- ref(metrics): Deprecate `send-metric` commands (#2154) by @szokeasaurusrex

## 2.36.1

### Various fixes & improvements

- Fix [a bug](https://github.com/getsentry/sentry-cli/issues/2148) where background Xcode uploads sometimes failed

## 2.36.0

### Various fixes & improvements

- Log when file not added to source bundle (#2146) by @szokeasaurusrex
- Bump Symbolic to `12.11.0`. This fixes a bug where uploading source files sometimes failed when any of the files were
  not UTF-8 encoded

## 2.35.0

### Various fixes & improvements

- fix: Only warn about mismatched URLs when they are different (#2143) by @szokeasaurusrex
- feat(proguard): Retry `upload-proguard` on 507 status (#2141) by @szokeasaurusrex

## 2.34.1

### Various fixes & improvements

- build: Bump symbolic to `12.10.1` (#2134) by @szokeasaurusrex
  - This includes an upstream bugfix for a bug that prevented debug file uploading with sources when any of the
    sources were not valid UTF-8.
- fix(debugIds): Always instantiate global `Error` class in debugId injection snippet (#2132) by @Lms24

## 2.34.0

### Various fixes & improvements

- fix(react-native): Fix RN 0.75 node binary build fail (#2131) by @krystofwoldrich
- feat: Prefer org auth token URL over manually provided URL (#2122) by @szokeasaurusrex
- meta: Update version pin README.md (#2123) by @szokeasaurusrex
- build: Update symbolic dependency to 12.10.0 (#2120) by @trzeciak

## 2.33.1

### Security fix

This release contains a fix for a bug where auth tokens would, under the following circumstances, be logged to `stdout`:

- The auth token was passed as a command line argument to Sentry CLI (via `--auth-token`)
- The log level was set to `info` or `debug`
  - The default log level is `warn`, so users using the default log level were unaffected by this bug

We now redact the `--auth-token` argument and anything else that looks like it might be an auth token when logging the
arguments that the Sentry CLI was called with (see #2115 and #2118 for details).

### Other fixes & improvements

- ref(token): Use secrecy crate to store auth token (#2116) by @szokeasaurusrex
- fix: Improve "project not found" message (#2112) by @szokeasaurusrex
- fix: Improve "release not found" message (#2112) by @szokeasaurusrex
- Fall back to co-location heuristic if sourcemap url appears remote (#1871) by @brettdh
- fix(sourcebundle): Skip non-UTF8 files (#2109) by @loewenheim

## 2.33.0

### Various fixes & improvements

- Recognize new format user tokens (prefixed with `sntryu_`) (#2100) by @szokeasaurusrex
- Fix regression in `files upload` (#2107)
- docs: Fix typos (#2102) by @kianmeng
- docs(id-support): Document that -p and -o arguments accept slugs and IDs (#2101) by @iamrajjoshi
- chore(deps): bump tar from 6.1.13 to 6.2.1 (#2027) by @dependabot
- build(deps): bump braces from 3.0.2 to 3.0.3 (#2088) by @dependabot
- build(deps): bump ws from 7.5.9 to 7.5.10 (#2091) by @dependabot

## 2.32.2

### Various fixes & improvements

- build: Bump `curl` to `0.4.46` in `Cargo.toml` (#2099) by @szokeasaurusrex
- build: Update `curl-sys` (#2075) by @szokeasaurusrex

## 2.32.1

Release performed for technical reasons. This release is identical to 2.32.0.

## 2.32.0

### Various fixes & improvements

- docs(commands): Add info about clap Derive API (#2066) by @elramen
- feat(metrics): Add send-metric command (#2063) by @elramen
- feat(auth): Allow global config to be located in XDG directory (#2059) by @elramen
- fix(commands): Add missing env vars for release name detection (#2051) by @elramen

## 2.31.2

### Various fixes & improvements

- Fix regressions related to `releases set-commits` command, which were introduced in 2.31.1.

## 2.31.1

### Various fixes & improvements

- fix(proguard): Don't require auth token with --no-upload flag (#2047) by @markushi
- fix(debug-files): Improve error when `check` passed a directory (#2034) by @szokeasaurusrex
- fix(xcode): Improve Xcode error msg when config load fails (#2028) by @elramen

## 2.31.0

With this change, dSYM uploads to the legacy endpoint and release file uploads are routed to the region URL directly (
e.g. to https://us.sentry.io instead of https://sentry.io). This change only affects users using the CLI to interact
with SaaS Sentry; everything stays the same for self-hosted users.

### Other changes

- build: `make lint` only with default features (#1994) by @szokeasaurusrex

## 2.30.5

Release made for technical reasons. There are no code changes in this version.

## 2.30.4

Release made for technical reasons. There are no code changes in this version.

## 2.30.3

### Various fixes & improvements

- fix: Handle .env errors (#1987) by @szokeasaurusrex

## 2.30.2

This release re-enables Python releases. There are no code changes.

## 2.30.1

### Various fixes & improvements

- fix(sourcemaps): allow complex file extensions (#1976) by @szokeasaurusrex

## 2.30.0

### Cron Monitor Changes

The `monitors run` subcommand now no longer accepts `--auth-token` or other means of authentication using token-based
auth. It is now required to use DSN based auth to monitor cron jobs using the sentry-cli.

## 2.29.1

Updated version 2.29.0 changelog. No code changes.

## 2.29.0

### Source maps fixes

This release fixes the behavior of `sourcemaps inject` and `sourcemaps upload`. We now treat minified and non-minified
source files the same way in both commands, which was always the desired behavior, and is also consistent with our JS
bundler plugins.

**Please be aware that from now on, `sourcemaps inject` will inject debug IDs into all JS source files at the path
provided to the command.** If you only wish for some of the files to have debug IDs injected, you need to modify the
path(s) passed to `sourcemaps inject` or you need to use the `--ignore` or `--ignore-file` options to exclude the files
you do not wish to inject the debug IDs into.

In the `sourcemaps upload` command, we have eliminated the "Minified Scripts" section in the Source Maps Upload Report.
Instead, these minified scripts will appear under "Scripts."

### Auth token validation

Sentry CLI now validates that you have provided an auth token whenever you run a command that requires authentication to
succeed. If you fail to provide an auth token when running such a command, the Sentry CLI will exit with an error
message explaining that the auth token is required but missing.

### Other fixes & improvements

- fix(sourcemaps): Add `.cjs` and `.mjs` to default `sourcemaps upload` extensions (#1961) by @szokeasaurusrex
- fix(xcode): Only parse Plist when required during RN source maps upload (#1940) by @krystofwoldrich
- fix(files): Fail when deleting all files fails with 404 error (#1949) by @szokeasaurusrex
- fix: support windows on arm via x64 binary (#1943) by @MarshallOfSound

## 2.28.6

### Various fixes & improvements

- fix(deploys): Revert "fix(deploys): Use `--project` argument (#1930)" (#1942) by @szokeasaurusrex

## 2.28.5

### Various fixes & improvements

- fix(deploys): Use `--project` argument (#1930) by @szokeasaurusrex

## 2.28.0

### New features

- New cron monitor configuration options (#1922) by @szokeasaurusrex
  - The `sentry-cli monitors run` command now has two new command line arguments: `--failure-issue-threshold` and
    `--recovery-threshold`. These arguments allow the user to specify the number of consecutive failed checkins that
    trigger an issue to be created and the number of successful checkins that trigger the issue to be resolved,
    respectively.

### Various fixes & improvements

- fix(sourcemaps): print source map URL instead of {source_url} on error (#1917) by @jbg

## 2.27.0

### Improvements

- Prefer `--url` argument over empty auth token URL (#1914) by @szokeasaurusrex
- feat(xcode): Print redirected output file path when going to the background (#1920) by @krystofwoldrich

### Fixes

- Correct error message for querying events/issues on non-existing project. The message now states that the project
  could not be found, instead of stating that the organization could not be found. (#1916) by @szokeasaurusrex

### Other

- Remove `dsyms/associate` API usage (#1886) by @Swatinem

## 2.26.0

### Various fixes & improvements

- meta: Revert "meta: Temporarily disable npm targets for v1 release (#1903)" (#1908) by @szokeasaurusrex
- Add "--environment" option to "monitors run" (supplement "-e" shorthand) (#1881) by @skalee
- meta: Temporarily disable npm targets for v1 release (#1903) by @lforst

## 2.25.3

- No documented changes.

## 2.25.2

- No documented changes.

## 2.25.1

- fix: Upload Xcode debug files and source maps background upload (#1896) by @krystofwoldrich
  - revert: Fixed a `curl` issue on Windows (#1815) by @xpirt

## 2.25.0

### Various fixes & improvements

- fix(api): Fix panic that occurred when `sentry-cli login` called with `--auth-token` (#1893) by @szokeasaurusrex

## 2.24.1

### Various fixes & improvements

- Revert #1885, which was causing an [error in the
  `sentry-cli login` command](https://github.com/getsentry/sentry-cli/issues/1888) (#1889) by @szokeasaurusrex

## 2.24.0

### Various fixes & improvements

- feat(sourcemaps): Improve `sourcemaps resolve` command output (#1880) by @szokeasaurusrex
- feat(api): Validate Auth Tokens client-side (#1885) by @szokeasaurusrex

## 2.23.2

### Various fixes & improvements

- ref: Removed unneeded identity `map` (#1883) by @szokeasaurusrex
- ci: Skip fallback download in tests (#1878) by @lforst
- ref: Emit better log message for fallback postinstall script (#1877) by @lforst
- fix: Manually download binary if optional dependency binary can't be found after installation (#1874) by @lforst
- fix(hybrid-cloud): Updates organization list to handle single org fanout (#1872) by @GabeVillalobos

## 2.23.1

### Various fixes & improvements

- ref: upgrade git2 so safe.directory is implemented more correctly (#1866) by @asottile-sentry
- fix(hybrid-cloud): Adds region fan-out to organizations list command (#1860) by @GabeVillalobos
- fix: install the x64 cli on windows-on-arm hosts (#1858) by @MarshallOfSound

## 2.23.0

### Various fixes & improvements

- build: Bump tempfile dependency (#1857) by @szokeasaurusrex
- build: Fix CLI in Xcode Cloud by Disabling Link-time Optimizations (#1855) by @szokeasaurusrex
- chore(deps): bump tough-cookie from 4.1.2 to 4.1.3 (#1675) by @dependabot
- chore(deps): bump word-wrap from 1.2.3 to 1.2.5 (#1856) by @dependabot
- feat(api): More descriptive `releases propose-version` error (#1854) by @szokeasaurusrex
- meta: Restructure Bug Report Issue Template (#1852) by @szokeasaurusrex
- fix(sourcemaps): don't attempt to treat remote URL as a local file path (#1850) by @brettdh
- feat(api): Validate `monitors run` command's `--timezone` argument (#1847) by @szokeasaurusrex

## 2.22.3

ref: Transition to binaries over npm (#1836)

## 2.22.2

This release contains no changes and was done for technical purposes.

## 2.22.1

This release contains no changes and was done for technical purposes.

## 2.22.0

- feat: Add opt-in code path to use binary distributions instead of downloaded CLI (#1835)
- build: Add optional dependencies to main cli package for binary distributions (#1834)
- feat: Publish binary distributions on npm (#1833)
- build: Add placeholder packages for npm binary distributions (#1828)

## 2.21.5

This release contains no changes and was done for technical purposes.

## 2.21.4

### Various fixes & improvements

- Improved error message if a project slug is missing (#1811) by @cleptric
- Fixed a `curl` issue on Windows (#1815) by @xpirt
- Added support for monitor upserts (#1807) by @szokeasaurusrex
- Fixed a bug in sourcemap `sourceRoot` handling (#1818) by @loewenheim

## 2.21.3

### Various fixes & improvements

- feat: Deprecate `sourcemaps explain` (#1797) by @loewenheim
- Allow log-level parameter to affect log from reading config (#1789) by @szokeasaurusrex
- feat: Add shell completion generation (#1775) by @NickyMeuleman
- fix: Log messages for invalid objects (#1771) by @loewenheim

## 2.21.2

### Various fixes & improvements

- fix(rn): Remove hermesc envs if hermes is disabled (#1754) by @krystofwoldrich
- fix: Don't fail on duplicate proguard release association (#1762) by @loewenheim
- fix: Make URL in org auth tokens optional (#1766) by @loewenheim
- Explain how to update in the README (#1767) by @philipphofmann

## 2.21.1

### Various fixes & improvements

- fix: Strip `/api/0` prefix from endpoint URLs (#1752) by @loewenheim

## 2.21.0

### Various fixes and improvements

- feat: `login` now opens the org auth token creation page (#1737) by @loewenheim
- feat: Debug IDs are now supported in Hermes bundles (#1667) by @krystofwoldrich
- feat: The root sourcemap for RAM bundles is now uploaded, improving support for preloaded modules (#1743) by
  @krystofwoldrich
- feat: Commands with a `--wait` flag now also have a `--wait-for <SECS>` option that additionally puts a limit on the
  wait time. (#1748) by @loewenheim
- deps: `symbolic` updated to 12.4.1 (#1749) by @loewenheim

## 2.20.7

### Various fixes & improvements

- fix(sourcemaps): Query parameters and fragments are removed from source mapping URLs (#1735) by @loewenheim
- fix(sourcemaps): Debug ID injection is significantly faster (#1736) by @loewenheim

## 2.20.6

### Various fixes & improvements

- fix: Always poll artifact assembly, even if nothing was uploaded (#1726) by @loewenheim
- feat(rn): Allow custom bundle command and Expo export:embed in xcode wrap calls (#1723) by @krystofwoldrich
- feat(bash-hook): Add support for `SENTRY_RELEASE` in bash-hook (#1720) by @boozec
- misc: Add CONTRIBUTING.md for working with integration tests (#1716) by @kamilogorek
- test: Prevent .sentryclirc being read from global config (#1716) by @kamilogorek
- test: Do not sign git tags and commits during tests (#1716) by @kamilogorek
- test: Make sure to always skip opening editor in git-based tests (#1716) by @kamilogorek

## 2.20.5

### Various fixes & improvements

- deps: Bump sourcemap to 6.4.1 (#1715) by @kamilogorek

## 2.20.4

### Various fixes and improvements

- fix(injection): Make debug id injection sound (#1693) by @loewenheim

## 2.20.3

### Various fixes & improvements

- fix(set-commits): Allow --initial-depth to be specified alongside --auto (#1703) by @kamilogorek
- feat: Allow the CLI binary path to be overridden (#1697) by @koddsson

## 2.20.1

### Various fixes and improvements

- fix: Correctly detect local binary when installing via npm (#1695)

## 2.20.0

### Various fixes and improvements

- feat(issues): Add `issues list` command (#1349)
- feat(inject): Make sourcemap discovery smarter (#1663)
- feat(config): Support organization-based tokens (#1673)
- feat(proguard): Create a weak release association with a proguard mapping file (#1688)
- fix(inject): Make pragma detection stricter (#1648)
- fix(inject): Mark paths argument as required (#1662)
- fix(debug-files): Add wasm to supported debug-files upload formats (#1683)
- fix(sourcemaps): Sourcemap rewriting no longer deletes debug ids when uploading (#1686)
- ref(sourcemaps): Do not dedupe artifacts if server won't handle it due to url limit (#1680)

## 2.19.4

### Various fixes and improvements

- feat: Don't upload chunks that are already on the server (fixed version) (#1660)

## 2.19.3

### Various fixes and improvements

- Revert "feat: Don't upload chunks that are already on the server (#1651)"

## 2.19.2

### Various fixes and improvements

- fix: Make artifact bundle creation deterministic (#1652)
- feat: Don't upload chunks that are already on the server (#1651)

## 2.19.1

### Various fixes & improvements

- fix(inject): Add semicolon at the end of inject snippet (#1643) by @kamilogorek

## 2.19.0

### Various fixes & improvements

- ref(debug-files): Use temp file handle instead of keeping zips in memory (#1641) by @kamilogorek
- feat(crons): Bring back legacy token-auth checkin API (#1634) by @evanpurkhiser
- ref: Unhide sourcemaps inject command as its already GA (#1631) by @kamilogorek
- ref: Match case-insensitive value when finding repo by name (#1629) by @maxnowack
- ref(inject): Inject code snippet at the start (#1567) by @loewenheim
- feat(crons): Replace API checkins w/ send_envelope (#1618) by @evanpurkhiser

## 2.18.1

### Various fixes & improvements

- ref(types): Add dedupe to available TS types (#1619) by @kamilogorek
- ref(debug-files): Polish after-upload message output (#1620) by @kamilogorek

## 2.18.0

### Various fixes & improvements

- ci: Use macos-latest to run build workflow (#1617) by @kamilogorek
- Update `sentry` SDK to fix sending raw envelopes (#1616) by @Swatinem
- misc: Add custom issues templates (#1613) by @kamilogorek
- ref: Remove proguard from debug-files upload types (#1615) by @kamilogorek
- ref(inject): Constrain file search by extensions (#1608) by @loewenheim
- feat(crons): Allow specifying checkin environment (#1599) by @evanpurkhiser
- chore(crons): Make monitors command visible (#1600) by @evanpurkhiser

## 2.17.5

### Various fixes & improvements

- feat: Print bundle id after artifact bundle creation (#1593) by @loewenheim
- ref: Use 'embedded sourcemap' label for upload log (#1595) by @kamilogorek

## 2.17.4

### Various fixes & improvements

- Add ability to send an Event as Raw Envelope (#1582) by @Swatinem

## 2.17.3

### Various fixes & improvements

- feat(js): Skip null as option same as undefined (#1579) by @mitsuhiko
- Add command `sentry-cli debug-files bundle-jvm` for bundling Java (and other JVM based languages) sources (#1551) by
  @adinauer

## 2.17.2

### Various fixes & improvements

- feat(debug_id): More determinism for JS debug IDs (#1577) by @mitsuhiko
- fix: inject only injects into js files (#1571) by @mitsuhiko
- feat: Add deterministic debug IDs for minified JS files (#1570) by @mitsuhiko

## 2.17.1

### Various fixes & improvements

- fix(build) Hotfix allowing extra binaries during wheel build (#1575) by @ashwoods
- fix(inject): Don't inject non-js files (#1572) by @loewenheim

## 2.17.0

### Various fixes & improvements

- feat(bash-hook): Add ability to specify tags for bash hook script (#1560)
- feat(sourcemaps): Add `--use-artifact-bundle` flag to force Artifact Bundles usage (#1557) (#1559)
- ref(sourcemaps): Check for release or debug ids before upload (#1561)
- ref(sourcemaps): Skip empty bundle uploads (#1552)
- ref(inject): Inject files with embedded sourcemaps (#1558)
- ref(inject): Inject regardless of whether a sourcemap is present (#1563)
- fix(inject): Use `File::create` to make sure files are truncated (#1562)

## 2.16.1

### Various fixes & improvements

- fix: Attach checkin payload to monitor requests (#1550) by @kamilogorek
- Use a deterministic bundle ID (#1546) by @mitsuhiko

## 2.16.0

### Various fixes & improvements

- ref(crons): Prefer DSN based auth (#1545) by @kamilogorek
- fix(inject): Improve fixup_js_file (#1533) by @loewenheim
- feat(inject): Handle relative and absoule sourcemap URLs (#1530) by @loewenheim
- ref(monitors): Prefer slugs over guids (#1540) by @evanpurkhiser
- feat(auth): Support DSN auth for the API client (#1536) by @evanpurkhiser
- ref(crons): monitors command is not legacy atm (#1538) by @evanpurkhiser
- fix(help): Consistent wording for API keys (#1539) by @evanpurkhiser

## 2.15.2

### Various fixes and improvements

- fix: Make sourcemap injection on upload opt-in (#1534)

## 2.15.1

### Various fixes and improvements

- fix: Properly overwrite the sourcemap when injecting (#1525)
- ref: Allow multiple paths in `sourcemaps inject` (#1523)

## 2.15.0

### Various fixes & improvements

- feat: Implement new chunk based upload for standalone artifact bundles (#1490)
- feat: Inject sourcemap debug ids by default when performing `sourcemaps upload` and print injection report (#1513)
- ref: Use recursive walk instead of globbing when looking for `sourcemaps inject` files (#1504)
- ref: When injecting sourcemap debug ids, make sure that `sourceMappingURL` comment is always kept at the end of a
  file (#1511)

## 2.14.4

### Various fixes & improvements

- fix: Include sources referenced but not embedded by an object file (#1486)
- chore: Reapply clap update to v4 and fix releases subcommands (#1500)
- deps: Update rust-sourcemap to 6.2.3 (#1502)

## 2.14.3

### Various fixes & improvements

- ref: fix macos wheel tags, add musllinux tags (#1497) by @asottile-sentry

## 2.14.2

- No documented changes.

## 2.14.1

### Various fixes & improvements

- fix: Revert clap-rs v4 update and add regression test for 'releases files upload-sourcemap' (#1496) by @vaind

## 2.14.0

### Various fixes & improvements

- ref: also distribute sentry-cli as a pip package (#1494) by @asottile-sentry
- ref: Support lower and uppercased env vars for npm cache (#1493) by @kamilogorek
- feat: Add sourcemaps inject command (#1469) by @loewenheim
- chore: update clap to v4 (#1489) by @vaind
- chore: update clap deprecations (#1487) by @vaind
- deps: Update symbolic to 12.0.0 (#1488) by @loewenheim
- feat: add new `debug-files print-sources` command (#1476) by @vaind

## 2.13.0

- feat: Added the `--raw` flag to `send-envelope` to allow sending envelopes without parsing or validation. (#1468)
- feat: extract and upload embedded Portable PDB from PE (#1463)
- ref(monitors): Drop passing `SENTRY_TRACE_ID` (#1472)
- ref(monitors): Rename MonitorStatus -> MonitorCheckinStatus (#1471)
- ref: Deduplicate artifacts upload using queried checksums (#1456)
- ref: Add better debug message for malformed config files (#1450)

## 2.12.0

### Various fixes & improvements

- feat: Enable portable PDB debug-files upload via Symbolic v11 (#1440)
- feat: Add support for Cloudflare Pages when detecting a release name (#1419)
- feat: Pass `SENTRY_TRACE_ID` to executed process in `monitors` (#1441)
- feat: Pass `SENTRY_MONITOR_ID` to executed process in `monitors` (#1438)
- fix: Prevent Sentry from attempting to access unbound variables in `bash-hook` (#1415)

## 2.11.0

### Various fixes & improvements

- feat: Report spawn failures for monitors by @mitsuhiko
- fix: Allow for path based sourcemaps in validation report @kamilogorek

## 2.10.0

### Various fixes & improvements

- feat: Add `--decompress` option to JS types (#1402) by @FSchieber
- fix: Always expand Plist vars during XCode upload (#1403) by @krystofwoldrich

## 2.9.0

### Various fixes & improvements

- feat: Replace `npmlog` dependency and restore support for Node v10 (#1392) by @mydea
- fix: Handle closed connection when fetching `sentry-cli` binary via JS script (#1389) by @kamilogorek
- fix: Re-add `server_name` in events via `contexts` sentry-rust feature (#1383) by @bengentil
- ref: Add length limit validation to `org` arguments (#1386) by @olksdr

## 2.8.1

### Various fixes & improvements

- fix: Replace usage of eval to obfuscate binary path from bundlers (#1374) by @timfish

## 2.8.0

### Various fixes & improvements

- fix: Use forward slash for Windows binary location path (#1369) by @timfish
- fix: Add + to reserved query characters in url encoding (#1365) by @kamilogorek
- feat: Add `headers` option to the JavaScript API (#1355) by @thibmeu
- fix: Add % and / to reserved path characters in url encoding (#1364) by @kamilogorek
- feat: Add support for Portable PDB debug files (#1345) by @Swatinem

## 2.7.0

### Various fixes & improvements

- fix: paths argument for debug-files find should take a value (#1346) by @kamilogorek
- chore: reuse danger workflow (#1332) by @vaind

## 2.6.0

### Various fixes & improvements

- fix: Update symbolic to fix spurious EOF errors (#1339) by @Swatinem
- fix: Break out of the loop when we reach cursor limit for list_release_files (#1337) by @kamilogorek
- ref: Provide better user messages for properties file configs (#1336) by @kamilogorek
- feat(sourcemaps): Add no-dedupe flag for skipping deduplication (#1334) by @kamilogorek
- misc: Add note about MSVC requirement (#1333) by @kamilogorek
- ref: Use better artifacts url resolution for sourcemaps explain (#1329) by @kamilogorek
- feat: Add top-level events command with list subcommand (#1307) by @dcariotti

## 2.5.2

### Various fixes & improvements

- fix: Use direct plist env vars fallback when called within xcode itself (#1311) by @kamilogorek

## 2.5.1

### Various fixes & improvements

- fix: Fallback to xcodebuild vars for faulty Info.plist files (#1310) by @kamilogorek
- ref: Log warning when debug source excedes item size limit (#1305) by @kamilogorek

## 2.5.0

### Various fixes & improvements

- feat: Allow for selecting frame in sourcemaps explain (#1293) by @kamilogorek
- misc: 1.74.5 changelog (#1291) by @kamilogorek
- chore: add missing changelog entry for 1.74.4 (#1289) by @vaind

## 2.4.1

### Various fixes & improvements

- ref: Print better error when processing appcenter paths (#1287) by @kamilogorek
- fix: Make sure release exists before querying for artifacts (#1284) by @kamilogorek

## 2.4.0

### Various fixes & improvements

- ci: Disable rustup self-update (#1278) by @kamilogorek
- feat: Add decompress flag to sourcemaps and files upload (#1277) by @kamilogorek
- feat: Use checksum to dedupe uploaded release artifacts (#1275) by @kamilogorek

## 2.3.1

### Various fixes & improvements

- ref: Dont print install progressbar for nonTTY and CI=1 (#1270) by @kamilogorek
- fix: move dist option to SentryCliUploadSourceMapsOptions (#1269) by @ikenfin

## 2.3.0

### Various fixes & improvements

- fix: Allow for using --auth-token with login command (#1266) by @kamilogorek
- deps: Update all Rust dependencies (#1265) by @kamilogorek
- fix: Increase TempFile robustness on Windows (#1256) (#1263) by @kamilogorek
- ref: Remove confusing ending dots from command logs (#1261) by @kamilogorek
- fix: Correct typo in sourcemaps explain output (#1258) by @huwq1987
- fix: Use first frame that has context-line in explain (#1255) by @kamilogorek
- feat: Add send-envelope command (#1254) by @kamilogorek

## 2.2.0

### Various fixes & improvements

- feat: Compute and upload il2cpp line mappings (#1248) by @loewenheim
- ref: Skip protected zip files when uploading debug files (#1245) by @kamilogorek

## 2.1.0

### Source Maps Upload Check "y-tho" (ongoing)

_Problem statement:_

Uploading source maps is a common source of frustration. Source maps are also one of the great value adds to our in
product experience. We want to automate supporting customers with frequent issues.

https://docs.sentry.io/platforms/javascript/sourcemaps/troubleshooting_js/

_Outcome: _

Developers will be provided with a tool to help them discover any issues they may have when uploading source maps

Sentry support will have a tool and docs to suggest to customers to hopefully first discover issues, and second at least
know what their problem is NOT.

_Key measurements:_

- qualitative: Is this useful for customers and support
- quantitative: Can we try to influence the number of Zendesk tickets
- quantitative: Can we influence the resolution time of source maps related Zendesk tickets

Can we find a way to track in zendesk the number of times the sentry-cli “y-tho“ functionality was useful

_Additional_

This is something users would run locally so I do not think we can track usage exactly what was not covered in y-tho

- Verify your source maps are built correctly
- Verify your source maps work locally
- Verify your source files are not too large
  - this is a fuzzy requirement today in sentry
- Verify artifacts are not gzipped
- Verify workers are sharing the same volume as web (if running self-hosted Sentry via Docker)
- Should spit out an easily readable and easily copy and paste - to put into ZenDesk or elsewhere for support colleagues

_Possible second milestone:_

https://github.com/getsentry/rust-sourcemap/tree/master/cli

- In sentry error incorrect source map location
- this helps when producing sourcemaps locally then line and column
- this verify that it resolves locally
  - if yes then it is a problem in between on sentry server side or upload
  - 1st Verifies what you upload to sentry is exactly what you upload to sentry
  - 2nd step from “y-tho” ensure previous steps are not for waste
- What is being automated?
  - on release page you have your files (release artificats)
    - download
    - manually check the line number matches the error
    - if correct then data is correct
    - then you know an error with cli and not with the source maps that were uploaded

By: @kamilogorek (#1235)

### Various fixes & improvements

- ref: Change comment format for find_matching_artifact (#1243) by @kamilogorek
- ref: Log correct AppCenter error message (#1243) by @kamilogorek
- fix: Respect no-zips option for debug files upload (#1239) by @saf-e
- chore: fix recommended VS Code extension name (#1240) by @vaind
- ref: Rename VERSION to SENTRY_CLI_VERSION in install readme (#1227) by @kamilogorek
- feat: Add organizations list command (#1213) by @kamilogorek
- docs(cli): Sync get-cli readme with our docs; add version specifier (#1225) by @kamilogorek
- test: Add integration tests for projects command (#1212) by @kamilogorek
- fix: replace git.io links with redirect targets (#1209) by @asottile-sentry

## 2.0.4

### Various fixes & improvements

- ref: Prevent @vercel/nft and similar tools from including binary file in their bundles (#1207) by @kamilogorek
- ref: Use node-fetch@v2 for the simplicity sake (#1206) by @kamilogorek

## 2.0.3

### Various fixes & improvements

- ref: Make `--header` a global flag so its position independent (#1194)
- ref: Restore `monitors` as hidden command w. integration tests (#1196)
- ref: Restore `bash-hook` as hidden command w. integration tests (#1197)

## 2.0.2

### Various fixes & improvements

- fix: Remove `fetch.FetchError` usage in favor of catch-all clause (#1193) by @kamilogorek
- ref: Restore and hide legacy aliases from 1.x for backward compatibility (#1192) by @kamilogorek

## 2.0.1

### Various fixes & improvements

- fix: Allow hyphenated release names with any flags position (#1187) by @kamilogorek

## 2.0.0

This is the first, long-overdue major release in over 5 years of sentry-cli's life.
Some APIs were removed, some reworked, some newly added.

Most of introduced API changes are backward compatible through hidden aliases, so there is no immediate need for users
developing 3rd party tools to make all the changes immediatelly.
We do however encourage everyone to do it sooner or later, as deprecated items will be removed in the next major
releases.

Breaking changes are denotated with _(breaking)_ tag, and appropriate required changes are provided for each entry.

### New APIs

- feat: Add `debug-files` command, which is a joined functionality of `difutil` and `upload-dif` commands.
- feat: Add `deploys` command, which was extracted from `releases deploys` subcommand.
- feat: Add `files` command, which was extracted from `releases files` subcommand.
- feat: Add `sourcemaps upload` command, which was extracted from `releases files upload-sourcemaps` subcommand.
- feat: Add `sourcemaps resolve` command.
- feat: Allow for specifying global `--header` argument, which supports multiple occurences, to configure outgoing
  requests
- feat: Implement global `--quiet`/`--silent` flags to allow silencing `stdout` output (This flag is currently
  implemented only for selected subcommands)

### Removed APIs

- ref: Remove `react-native codepush` subcommand (use `react-native appcenter` instead) _(breaking)_
- ref: Remove `react-native-gradle` and `react-native-xcode` commands (use `react-native gradle` and
  `react-native xcode` instead) _(breaking)_
- ref: Remove `crash_reporting` related code and `with_crash_reporting` crate feature (no required changes) _(breaking)_
- ref: Remove `SENTRY_NO_PROGRESS_BAR` env var in favor of `SENTRYCLI_NO_PROGRESS_BAR` (rename env variable) _(
  breaking)_
- ref: Hide `difutil id` subcommand (use `debug-files check` instead)
- ref: Hide `upload-dsym` command (use `debug-files upload` instead)
- ref: Make `releases upload-sourcemaps --rewrite` a default behavior now
- ~ref: Remove `upload-dsym` command (use `debug-files upload` instead) _(breaking)_~ _restored in 2.0.2 as hidden
  alias_
- ~ref: Remove `difutil id` subcommand (use `debug-files check` instead) _(breaking)_~ _restored in 2.0.2 as hidden
  alias_
- ~ref: Remove `monitors` command (support for this feature has been dropped) _(breaking)_~ - _restored in 2.0.3 as
  hidden command_
- ~ref: Remove `bash-hook` command (use `1.x` if you still need the functionality) _(breaking)_~ - _restored in 2.0.3 as
  hidden command_

### Breaking Changes

- ref: Update minimal required `node` version to `v12` (update node version) _(breaking)_
- ref: Rename `--header` argument of `releases files upload` command to `--file-header` (rename flag) _(breaking)_
- ref: Rename `CUSTOM_HEADER` to `SENTRY_HEADER` and `defaults.custom_header` to `http.header` (rename env variable or
  update config file) _(breaking)_
- ref: Make `ignore-empty` for `releases set-commits` a default behavior and hide `--ignore-empty` flag (remove
  `--ignore-empty` usage) _(breaking)_

### Various fixes & improvements

- feat: Implement `--quiet` flag for `releases upload-sourcemaps` command
- feat: Implement `--quiet` flag for `difutil check` command
- ref: Make `--auth-token` a global argument
- ref: Make all `ProgressBar` instances and logs always write to `stderr`
- ref: Migrate error handling from `failure` to `anyhow` crate

## 1.74.5

### Various fixes & improvements

- deps: Add resolution to bump `ansi-regex` to version `^3.0.1` (#1281)
- ref: Increase `TempFile` robustness on Windows (#1256)

## 1.74.4

### Various fixes & improvements

- ci: Add merge target (f9a2db3) by @kamilogorek
- ref: Prevent @vercel/nft and similar tools from including binary file in their bundles (#1207) by @kamilogorek

## 1.74.3

### Various fixes & improvements

- deps: Update symbolic and git-rs crates (#1170) by @kamilogorek
- ref: Filter non-error goblin logs (#1169) by @kamilogorek
- ref: Use provided version in release log messages (#1162) by @kamilogorek

## 1.74.2

### Various fixes & improvements

- revert: ref: Dont run install script using node binary #1151 (#1155) by @kamilogorek

## 1.74.1

### Various fixes & improvements

- ref: Make org and project flags position independent (#1153) by @kamilogorek

## 1.74.0

### Various fixes & improvements

- ref: Dont run install script using node binary (#1151) by @kamilogorek
- feat: Add show-projects and show-commits flags to 'releases info' command (#1144) by @kamilogorek
- ref: Rework find_id function of difutil find (#1149) by @kamilogorek
- ref: Rework find_matching_rev tests to not use our real repo history (#1147) by @kamilogorek
- deps: Update git2 crate to 0.14.1 to support custom git extensions (#1140) by @kamilogorek
- ci: Change stale GitHub workflow to run once a day (#1141) by @kamilogorek

## 1.73.2

### Various fixes & improvements

- install: Rename SENTRY_NO_PROGRESS_BAR flag to SENTRYCLI_NO_PROGRESS_BAR (#1132) by @kamilogorek

## 1.73.1

### Various fixes & improvements

- feat: Allow for using local binary through SENTRYCLI_USE_LOCAL env (#1129) by @kamilogorek
- ref: Dont panic on malformed xcodeproj directories (#1127) by @kamilogorek

## 1.73.0

- feat: Add checksum validation for installed binaries (set `SENTRYCLI_SKIP_CHECKSUM_VALIDATION` to opt-out) (#1123)
- fix: Detect unwind and debug information in files linked with `gold` (#1124)
- ref: Silence progress bar in CI environments by default (#1122)

## 1.72.2

- feat: Use default xcode values for plist struct (#1111)
- fix: Fixes a panic when inspecting debug files larger than 4GB (#1117)
- ref: Update log message when bundle ID is missing (#1113)

## 1.72.1

- fix: Dont include `debug_id` during assemble when not PDBs are not supported (#1110)
- ref: Remove all release files instantaneously with `--all` flag (#1108)

## 1.72.0

- feat: Add `CUSTOM_HEADER` support to JS wrapper (#1077)
- feat: Add `SENTRYCLI_SKIP_DOWNLOAD` flag for preventing download (#1074)
- feat: Allow for configuring max item size for dif bundles (#1099)
- fix: Prevent daemonize mode from crashing upload process (#1104)
- fix: Restore logger initialization (#1102)
- ref: Box `ParseDif::Object` value to prevent large enum variant (#1094)
- ref: Rename ini group from `dsym` to `dif` with a fallback (#1103)
- ref: Show `project` flag for releases command (#1065)

## 1.71.0

- feat: Add optional positional argument to `send-event` that allows to specify a path to JSON serialized events (#1058)
- fix: Handle `SENTRY_CLI_NO_EXIT_TRAP` unbound variable (#1059)

## 1.70.1

- feat: Add `SENTRY_CLI_NO_EXIT_TRAP` flag to control EXIT trap in `bash-hook` command (#1050)
- fix: Remove warning about relative urls for chunk uploads (#1054)
- fix: Typo in `vscRemote` TS type (#1052)
- fix: Use internal timer for ProgressBar duration (#1055)
- ref: Update dockerfile alpine image (#1057)

## 1.70.0

- feat: Add `no-upload` flag for `upload-dif` command (#1044)
- feat: Add support for glob patterns in `upload-sourcemaps` command (#1048)
- feat: Allow to load dotenv from non-standard path through `SENTRY_DOTENV_PATH` (#1046)
- fix: Follow symlinks when traversing during sourcemaps upload (#1043)
- ref: Use `SOURCE_VERSION` first prior to `HEROKU_SLUG_COMMIT` in Heroku (#1045)

## 1.69.1

- misc: Re-release of `1.69.0` due to malformed artifacts

## 1.69.0

- feat: Print upload context details (#1016)
- feat: Allow for changing log stream through `SENTRYCLI_LOG_STREAM` variable (#1010)
- fix: Set archString to `armv7` when`arch="arm"` (#1024)
- fix: Dont render progress bar when content length is missing (#1011)
- fix: Do not supply `debug_id` for object files (#981)
- ref: Update `symbolic` to `8.3.1` (#1033)
- ref: Hide `upload-symbol-maps` flag for `upload-dif` command (#1017)

## sentry-cli 1.68.0

- feat: Add ability for `include` in JS sourcemap upload options to be an object (#1001)

## sentry-cli 1.67.2

- fix: Correctly resolve paths with hashes in `url_to_bundle_path` (#1000)
- ref: Provide JSDocs for TS typings, change `started/finished` to `number|string`, add `ignoreEmpty` to JS API (#999)
- ref: Add `npm_config_cache_folder` to function of getting cache (#998)

## sentry-cli 1.67.1

- feat: Print sourcemaps/files operations timings information (#995)

## sentry-cli 1.67.0

- feat: Add `--ignore-empty` flag to `releases set-commit` command, that will not bail command when no patchset is
  created (#993)
- feat: Add `--raw` and `--delimiter` flags to `releases list` command (#994)

## sentry-cli 1.66.0

- feat: Allow to define a custom `release` and `dist` for XCode SourceMaps upload (#967)
- feat: Support custom request header through `CUSTOM_HEADER` env variable and `http.custom_header` config (#970)
- fix: Add missing `ignoreMissing` flag for `setCommit` command to JS API (#974)
- fix: Change ignore-missing to flag with 'long' modifier (#965)

## sentry-cli 1.65.0

- feat: Allow for ignoring missing commits in set-commit with `--ignore-missing` flag (#963)
- feat: Support BCSymbolMap uploading (#952)

## sentry-cli 1.64.2

- ref: Rely on spawn process error for detecting command presence (#958)

## sentry-cli 1.64.1

- fix: Redirect spawned JS process outputs to `/dev/null` instead of filling up pipe buffers (#949)
- ref: Allow `upload-dif` to follow symlinks to make it inline with `difutil` behavior (#948)

## sentry-cli 1.64.0

- feat: Add TypeScript `SentryCli` types (#934)
- ref: Provide a helpful error messages for xcode/codepush/appcenter binary calls (#937)

## sentry-cli 1.63.2

- feat: List logging levels on CLI output (#926)
- fix: Update proguard version (#927)
- fix: Run update nagger only for versions lower than current one (#925)
- fix: Add some npm logging to aid in troubleshooting (#921)

## sentry-cli 1.63.1

- fix: Correctly detect hidden Swift symbols (#918)
- fix: Rename `arm64` as `aarch64` in install script (#917)
- fix: Verify CLI installation before launching (#916)

## sentry-cli 1.63.0

- build: ARM for Linux (#890)
- ref: `is_outdated` should not report when release contains version older than latest (#899)

## sentry-cli 1.62.0

- fix: Detect debug information in MIPS binaries (#897)
- fix: Use `http_proxy` config value in the handler directly (#893)
- fix: Limit chunk upload waiting to 5 minutes (#896)
- ref: Prefer universal binaries in homebrew (#879)
- ref: Prefer universal binaries on macOS (#878)
- build: macOS arm64 on stable Rust (#884)
- build: Build universal macOS binary on macos-latest (#877)

## sentry-cli 1.61.0

- fix: Add missing underscores for template in bash hook (#872)
- feat: macOS builds for `arm64` and universal binaries (#873)

Sentry-cli will not upgrade to the `arm64` build automatically, if you're currently running on Rosetta 2. To install the
`arm64` version:

- Please ensure that your terminal and shell both run natively without emulation. You can check this by running
  `uname -m` in your terminal.
- Remove your existing installation of `sentry-cli`.
- Follow the [Installation Instructions](https://github.com/getsentry/sentry-cli#installation) for a fresh installation.

## sentry-cli 1.60.1

- fix: Restore release modification calls to use put request, while preserving restore/archive capability (#870)

## sentry-cli 1.60.0

- feat: Added support for WASM debug info files (#863)

## sentry-cli 1.59.0

- feat: Allows the user to specify multiple projects for a release (#842)
- feat: Add cli arg to override sentry-cli command in bash-hook (#852)
- ref: Remove --rewrite flag and make it a default (#853)

## sentry-cli 1.58.0

- feat: Expose environment configuration in javascript (#830)
- ref: Use better error messages for install script (#833)

## sentry-cli 1.57.0

- feat: Allow for passing custom timestamp with `send-event` (#826)
- fix: OS arch detection for `IA32` (#824)

## sentry-cli 1.56.1

- fix: Use updated release name format during upload process (#818)

## sentry-cli 1.56.0

- feat: Add support for architectures other than x86/x64 when running installation script (#811)
- feat: Add `--confirm` flag to skip confirmation prompt during uninstall command (#812)
- misc: Upgrade symbolic to `7.5.0` (#813)

## sentry-cli 1.55.2

- fix: Path handling edgecases for `upload-dif` (#795)
- fix: Dont limit commits count for release updates (#808)
- ref: Update Android/iOS releases format (#805)

## sentry-cli 1.55.1

- feat: add support for CicleCI (#784)
- fix: Default to sending local commits if no repos linked (#791)
- ref: Make Update Nagger less aggressive (#793)

## sentry-cli 1.55.0

- feat: Release files batch upload (#715)
- feat: Add pipeline env variable option and include it in UA string (#774)
- feat: Upload formatted commit metadata from local git tree to Sentry for a release (#776)
- feat: Provide flag for allowing failures in monitor command (#780)
- fix: Do not error when offsetting too far on enumeration (#746)
- fix: Update base "alpine" image in Dockerfile (#757)
- fix: Allow for release names with leading hyphen (#770)
- fix: Handle teamless projects correctly (#773)
- fix: Parse BitBucket Server integration repo url correctly (#775)
- ref: Switch from symbolic::proguard to proguard crate (#756)

## sentry-cli 1.54.0

- feat: Add `--no-environ` parameter to `bash-hook` (#745)
- feat: Allow for disabling install progress-bar without silencing npm using `SENTRY_NO_PROGRESS_BAR` env var (#754)
- fix: Use correct required option to `newDeploy` JS api (#755)

## sentry-cli 1.53.0

- feat: `releases deploys` JavaScript API (#741)
- fix: `--log-level` should be case insensitive (#740)

## sentry-cli 1.52.4

- fix: Dont panic on unknown log level (#733)
- ref: Use temp dir to store jsbundle maps (#737)

## sentry-cli 1.52.3

- fix: Correctly store child process before attaching handlers (#718)

## sentry-cli 1.52.2

**This release sets `node.engine: >=8` which makes it incompatible with Node v6**
If you need to support Node v6, please pin your dependency to `1.52.1`
or use selective version resolution: https://classic.yarnpkg.com/en/docs/selective-version-resolutions/

- feat: Support Google Cloud Builder VCS detection (#481)
- fix: Mark files as unusable withid (#709)

## sentry-cli 1.52.1

- fix: Respect `configFile` for release commands invoked through JS API (#700)

## sentry-cli 1.52.0

- feat: Add an optional argument to override the entire release name for a CodePush release (#692)
- feat: Introduce `g/global` flag for `login` command (#690)
- feat: Add support for `INFOPLIST_OTHER_PREPROCESSOR_FLAGS` (#682)
- feat: Detect CodeBuild slug for `propose-version` (#681)
- feat: Show project and organization when using info log level (#670)
- feat: Add `bitbucket_server` to reference url check (#668)
- fix: Log config path only when its actually loaded (#677)
- fix: Make sure that requests are not authenticated twice and warn for rel urls (#675)
- fix: Override local `env.SENTRY_PROPERTIES` rather than global `process.env` (#667)
- fix: `react-native` xcode uses regex to detect Debug builds (#665)
- meta: Add Linux support to the Homebrew formula (#674)

## sentry-cli 1.51.1

- fix: Skip files larger than 1MB (#662)

## sentry-cli 1.51.0

- feat: Add `dist` option to `react-native appcenter` command (#653)
- ref: Notify user about missing `sudo` command instead of incorrect "No such file or directory" when
  updating/uninstalling `sentry-cli` (#656)
- fix: Remove redundant `Closing connection 0` warnings after every HTTP request (#657)
- fix: Update release structure for XCode React Native calls (#660)

## sentry-cli 1.50.0

- feat: Allow setting of `git` remote (#637)
- feat: Expose code IDs from `difutil` check (#639)
- feat: Implement workarounds for dealing with hermes bytecode (#646)
- feat: Allow for `--silent` flag in installation script (#648)
- feat: Support `dist` option in JS API (#642)
- ref: Treat `301/302` `upload_chunks` response codes as errors (#651)
- fix: Add `Content-Length=0` header to reprocessing POST request (#647)

## sentry-cli 1.49.1

- Add support for `git://`, `git+ssh://`, and `git+https?://` VCS repos (#636)
- Allow overriding dist in Xcode (#627)
- Skip pch and large files in source bundles (#624)

## sentry-cli 1.49.0

- Detect Heroku's `SOURCE_VERSION` environment variable (#613)
- Allow extensions with dots for sourcemap uploads (#605)
- Fix validation of `releases set-commits` options in JS (#618)
- Add an optional column for project slugs in `releases list` (#612)
- Add an optional `--wait` flag for upload-dif (#619)

**NOTE**: This release changes the default behavior of `upload-dif`. Previously,
the command waited until Sentry had fully processed uploaded files. Now, the
command terminates after a successful upload but does not wait for server-side
processing. This will speed up uploads for the common case. Specify `--wait` for
the old behavior if you want to make sure that debug files are available before
sending native events.

## sentry-cli 1.48.0

- Add support for Brotli, GZip and Deflate compression algorithms for binary download (#607)
- Fix binary download progress bar calculations (#606)

## sentry-cli 1.47.2

**Changes**:

- Always show the full version in `releases list` (#584).
- Do not warn when using the standard docker entrypoint.

**JavaScript API**:

- Pass the `silent` option to `releases` commands in JavaScript (#552).
- Allow setting commits on a release in JavaScript (#580).

**Fixed bugs**:

- Fix an error in the bash hook if the log file gets deleted (#583).
- Fix detection of Azure repositories in `releases set-commits` (#576).
- Fix detection of annotated tags in `releases set-commits` (#598).
- Fix normalization of sourcemap URL prefixes with trailing slashes (#599).
- Fix upload of source bundles created with `difutil bundle-sources` (#602).

## sentry-cli 1.47.1

- Fix potentially broken payloads in `send-event`.

## sentry-cli 1.47.0

- Trim whitespace in header values to prevent potential header injections
  through the auth token header. (#563)
- Improved Azure DevOps URL parsing. (#556)

## sentry-cli 1.46.0

- Relax the release file limit for sourcemap uploads when artifact bundles
  are supported by the serntry server (#559)

## sentry-cli 1.45.0

- Allow ports in VCS urls when associating commits (#551)
- Support PDB and PE uploads to Sentry (#553)

## sentry-cli 1.44.4

- Emit better version names for react native (#506)
- Fix a regression in sourcemap uploads for certain release names (#549)
- Ensure case insensitive git repository matching (#511)

## sentry-cli 1.44.3

- Fix a regression with URL prefixes in sourcemap uploads (#544)

## sentry-cli 1.44.2

- Even faster sourcemap uploads to sentry.io (#540, #542)

## sentry-cli 1.44.1

- Fixed a segfault in curl on empty file uploading (#535)

## sentry-cli 1.44.0

- Parallelize source map uploads (#533)

## sentry-cli 1.43.0

- Add support for File RAM Bundles (#528)
- Accept more Azure DevOps URLs (#525)

## sentry-cli 1.42.0

- Add support for Indexed RAM Bundles (#523)
- Add "silent" option to JS constructor (#512)

## sentry-cli 1.41.2

- Fix slow unzipping in debug file upload (#519)

## sentry-cli 1.41.1

- Warn before uploading more than 20.000 files to a release (#513)

## sentry-cli 1.41.0

- Recognizes GNU compressed debug files on Linux
- Also uploads Breakpad files and ELF files only containing symbol tables

## sentry-cli 1.40.0

- Automatically retry on various socket and SSL errors (#466, #490)
- Use a connection pool for the outgoing API requests. This is likely to resolve
  some issues in curl itself that manifested itself as malloc errors on shutdown (#489)
- Upgrade internal dependencies and shrink overall binary (#488)
- Upgrade internal sentry crate

## sentry-cli 1.39.1

- Fix Proguard upload issues on Windows (#484).

## sentry-cli 1.39.0

- Release enabling an internal sentry experiment.

## sentry-cli 1.38.1

- Fix plist parsing

## sentry-cli 1.38.0

- Upgraded symbolic which offers support top R8 code shrinker.

## sentry-cli 1.37.4

- Added `SENTRY_NO_PROGRESS_BAR` environment variable to suppress progress
  bars (#467)
- Fixed an issue where dif uploads would indicate failure if no files where
  to upload.

## sentry-cli 1.37.3

- Report non zero status for server side processing errors on dif upload (#465)
- Improve error messages for 502/504 (#459)
- Relax VCS url comparisions to improve on-prem support

## sentry-cli 1.37.2

- Retry on upload-related operations (chunk-upload, assemble) (#456)
- Add new anylog version (#455)

## sentry-cli 1.37.1

- Fix the detection of debug information in ELF files (#437)
- Add support for ELF files in the `difutil` commands (#447)
- Speed up `sentry-cli update` by using the Sentry release registry (#438)
- Dump http requests in debug mode for better debugging (#448)

## sentry-cli 1.37.0

- Support React Native >= 0.46 (@stephan-nordnes-eriksen, #377)
- Cache binaries to speed up NPM package installation (@timfish, #425)
- Check for successful upload of debug files (#429)
- Limit debug file uploads to 2GB (maximum allowed by Sentry) (#432)

## sentry-cli 1.36.4

- Add support for GitLab in `releases set-commits` (#419)
- Fix a bug where uploaded debug files might show up as _"Generic"_ (#420)

## sentry-cli 1.36.3

- Print out how sentry-cli was invoked in debug log

## sentry-cli 1.36.2

- Download packages from Fastly's CDN when installing via NPM and Brew (#417)
- Allow uploading executables and debug symbols in one go (#412)

## sentry-cli 1.36.1

- Fixes a bug that prevented listing and creating releases

## sentry-cli 1.36.0

- Show project IDs in project listing (#384)
- Fetch all projects, repos and releases if you have more than 100 (#388, #390)
- Support debug symbols with DWARF 5 debug information (#389)
- Fix `--no-environ` parameter in `send-event` (#391)
- Remove a misleading success message in `send-event` (#397)
- Improve debug logs and error output (#393, #394, #399)

## sentry-cli 1.35.6

- Fix a bug introduced with the `--url-suffix` option in `upload-sourcemaps`
- Fix broken commit detection for releases (#378, #381)

## sentry-cli 1.35.5

- Add `--url-suffix` option for `upload-sourcemaps` (#373)

## sentry-cli 1.35.4

- Additional compatibility improvements for the Docker image (#368)

## sentry-cli 1.35.3

- Add a warning about new Docker entrypoint (#367)

## sentry-cli 1.35.2

- Change entrypoint for Docker image (#358)
- Use `perl` over `strftime` in bash hook (#359)
- Fix iTunes Connect BCSymbolMap handling in `upload-dif` (#362)
- Display error messages when re-uploading broken DIFs (#363)

## sentry-cli 1.35.1

- Resolve a hang on certain windows versions on shutdown (#349)

## sentry-cli 1.34.0

- Improve the error message for renamed projects (#330)
- Fix appcenter commands on Windows (#331)
- Fix grammar in some help texts (#337, @gorgos)
- Fix frozen upload-dif on some Windows versions (#342)

## sentry-cli 1.33.0

- Add support for AppCenter CLI for codepush releases (#327)
- Deprecate the codepush CLI command (#327)
- Fix a bug where commands would fail with connection errors

## sentry-cli 1.32.3

- Skip invalid ZIP files during debug file upload (#320)
- Generate better error messages for renamed projects (#321)

## sentry-cli 1.32.2

- Compress debug symbols for faster uploads (#315)
- Refactor `send-event` to include more consistent information (#316, #318)

## sentry-cli 1.32.1

- Improve update prompts (#306, @danielcompton)
- Support event environments in bash hook (#312, @geniass)
- Use `DWARF_DSYM_FOLDER_PATH` in upload-dsym (#313)
- Skip malformed object files during upload scan (#313)

## sentry-cli 1.32.0

- Drop support for older macOS versions to work around an old xcode linker bug

## sentry-cli 1.31.2

- Disabled automatic crash reporting

## sentry-cli 1.31.1

- Fixed out of bounds panic for sourcemaps without sources (#299)
- Fixed commit detection when VSTS was used (#300)

## sentry-cli 1.31.0

- Restrict file permissions for newly created `.sentryclirc` (#296)
- Fix `SENTRY_DSN` environment variable parsing for `send-event` action (#292)
- Build statically linked `musl`-based binaries for Linux (#294)
- Detect `HEROKU_SLUG_COMMIT` in propose-version (#298)

## sentry-cli 1.30.5

- Add better error diagnostics (internal change, #288)

## sentry-cli 1.30.4

- Show correct identifiers when uploading Windows symbols (#280)

## sentry-cli 1.30.3

- Attempted to make the windows executable more portable (#269)
- Fixed the JavaScript API (#270)
- Fixed a bug where breadcrumbs were not always sent (#268)

## sentry-cli 1.30.2

- Fixed #252

## sentry-cli 1.30.1

- Expose `execute` on SentryCli js wrapper

## sentry-cli 1.30.0

- Improve the upload for debug information files. It is now faster, allows to resume after network errors, and supports
  much larger files.
- Add commands to upload Breakpad and ELF (Linux) symbols. See
  our [documentation page](https://docs.sentry.io/learn/cli/dif/) for more information.
- Fix JavaScript tests on Windows

## sentry-cli 1.29.1

- Fix NPM installation on Windows

## sentry-cli 1.29.0

- **BREAKING**: Drop support for Node 0.12. Please pin version `1.28.4` or install sentry-cli using
  a [different method](https://docs.sentry.io/learn/cli/installation/#automatic-installation) if you still require Node
  0.12.
- Fix NPM installation behind proxies
- Remove console output when using the JS interface

## sentry-cli 1.28.4

- Revert `Info.plist` handling to pre-`1.27.1` as it was causing issues when the `"Preprocess Info.plist File"` setting
  was turned on in Xcode
- Include CA certificates in the Docker container

## sentry-cli 1.28.3

- Reverted new config handling because of problems it caused.

## sentry-cli 1.28.2

- Fixed use of `SENTRYCLI_CDNURL` to override the npm download URL. See
  the [documentation](https://docs.sentry.io/learn/cli/installation/#installation-via-npm) for more information
- Better handling of environment variables and config files. Please let us know if one of your configuration files or
  environments doesn't get recognized anymore after the update
- The official docker image is now smaller and does not require dependencies anymore
- Replaced confusing errors when using `codepush` with hints to resolve the error

## sentry-cli 1.28.1

- Expose getPath() to not break setups

## sentry-cli 1.28.0

- Change JS bindings to be conform with the cli interface
  Please note that this is a breaking change if you used the JS interface before.

## sentry-cli 1.27.1

- Read from the correct `Info.plist` in XCode builds, courtesy of @adbi
- Allow to specify device family and model in `send-event`, courtesy of @kirkins
- Supply environment variables when using the JavaScript API
- Allow to override the NPM download URL via `SENTRYCLI_CDNURL` environment variable

## sentry-cli 1.27.0

- Support all options in the JS binding for `upload-sourcemaps`, courtesy of @montogeek
- Enable automatic IP addresses when sending events with `send-event`, courtesy of @kirkins
- No longer require secret keys to send events with `send-event`
- Improve and speed up debug symbol handling in `upload-dsym`

## sentry-cli 1.26.1

- Faster discovery of debug symbols in `upload-dsyms`
- Fix a bug in sourcemap uploading via JS, courtesy of @roelvanhintum
- Security update to OpenSSL 1.0.2n for Linux builds
- Fix a SSL verification command line flag

## sentry-cli 1.26.0

- The npm package has moved to [`@sentry/cli`](https://www.npmjs.com/package/@sentry/cli)
- Installing with npm on Windows now downloads the 64-bit version
- Exit with a proper error code when `send-event` fails, courtesy of @kirkins
- More informative errors on failed API requests
- No more annoying update reminders in the Docker images

## sentry-cli 1.25.0

- Do not run update nagger if the command is not connected to a terminal
- Source map uploading now correctly determines sourcemap references even if the rewrite
  flag is not passed.
- There is an offical Docker image with `sentry-cli` preinstalled:
  `docker run --rm -it -v $(pwd):/work getsentry/sentry-cli sentry-cli --help`
- Added support for automatically determining corvoda releases.

## sentry-cli 1.24.1

- Fix an issue with bash hooking not working if sentry-cli was installed on a path
  containing whitespace

## sentry-cli 1.24.0

- Improved sending events from bash. See
  [Sending Events](https://docs.sentry.io/learn/cli/send-event) for more information
- Hook into bash and send events for failed commands automatically. See
  [Bash Hooks](https://docs.sentry.io/learn/cli/send-event/#bash-hook) for more
  information
- Set `SENTRY_LOAD_DOTENV=0` to disable automatic loading of `.env` files
- Fix an issue where `info.plist` files were not resolved in XCode projects
- Fix an issue where the `PROJECT_DIR` environment was not used correctly

## sentry-cli 1.23.0

- Fix a bug that prevented uploads of ProGuard mapping files on Windows
- Improve command and parameter descriptions (`--help`)
- Updated dependencies

## sentry-cli 1.22.0

- Add `--ignore` and `--ignore-file` parameters to `upload-dsyms`
- Fix some typos in the CLI (thanks @mbudde and @AdrienDS)

## sentry-cli 1.21.0

- Fix codepush command for android
- Fixed added bitbucket provider support #115

## sentry-cli 1.20.0

- Updated dependencies
- Added encoding detection for javascript files
- Added bitbucket provider support
- Fixed an issue where codepush was not passing the right plist to the parser

## sentry-cli 1.19.1

- Resolved an issue where sourcemaps were not uploaded (#112)

## sentry-cli 1.19.0

- Added support for preprocessor `info.plist` files
- Unified `info.plist` handling in all places
- Added basic validation for the API URL to avoid common user errors
- Resolved an issue with NPM releases on ES5 environments
- Resolved an issue where `releases propose-version` incorrectly required an org to be
  passed
- Added support for handling `BCSymbolMap` files when uploading dsym files

## sentry-cli 1.18.0

- Ensure parent directories exist when writing Proguard meta properties.
- Write Proguard properties even if upload is disabled.
- Reject leading/trailing spaces in releases.

## sentry-cli 1.17.0

- Made npm install compatible with ES5
- Solved a potential issue with spaces in file paths for npm installation
- Added automatic update check (can be disabled with `update.disable_check` in the config
  or the `SENTRY_DISABLE_UPDATE_CHECK` environment variable)
- Resolved a crash when uploading empty files
- Lowered default symbol upload size to work around some server limitations

## sentry-cli 1.16.0

- added ability to upload proguard files with a forced UUID
- added `difutil uuid` command to print the UUID(s) of a mapping file to stdout

## sentry-cli 1.15.0

- Improved the `no-upload` flag to proguard upload
- Added debug info files debug commands

## sentry-cli 1.14.0

- Added support for disabling desktop notifications (only affects xcode builds so far)
- Added support for uploading proguard files on supported sentry server versions

## sentry-cli 1.13.3

- Fixed installation for npm

## sentry-cli 1.13.2

- Put `sentry-cli.exe` directly into the `bin/` folder on windows for npm installations

## sentry-cli 1.13.1

- Fixed another issue with yarn redownloading binaries

## sentry-cli 1.13.0

- Added `dist` support for send-event
- Improved download script for npm installs to not download unnecessarily with yarn.

## sentry-cli 1.12.0

- Added support for explicit bundle IDs for codepush releases
- Added `--print-release-name` to print out the release name for codepush releases to the
  command line to improve scripting capabilities
- Extended `propose-version` for releases to support iOS and android release names if
  projects are automatically discovered
- Parse grade files instead of android manifests for version and bundle IDs for release
  detection
- Fix broken xcode notifications when projects where opened from the command line
- Fixed limitations in automatically detecting the bundle IDs for xcode projects

## sentry-cli 1.11.1

- Resolved an issue where sourcemap uploading failed when empty files were encountered

## sentry-cli 1.11.0

- Initial work for codepush support (pending support in `react-native-sentry`)
- Moved `react-native-xcode` to `react-native xcode`
- Added support for `${FOO}` style variable expansion in xcode

## sentry-cli 1.10.2

- Fixed an issue for windows npm installation
- Stop generating a debug log file in `/tmp` for npm on unixes

## sentry-cli 1.10.1

- fixed a bug that caused the npm install to fail

## sentry-cli 1.10.0

- Added user support for `send-event`

## sentry-cli 1.9.2

- Improved logging facilities
- Fixed npm installation on windows

## sentry-cli 1.9.1

- Changes sourcemap rewriting to ignore bad files on source inlining.
- Fixed a bug in the JSON output of the `info` command.

## sentry-cli 1.9.0

- Added support for referring to previous hashes in `set-commits` with `OLD_REV..NEW_REV`
- Resolve tags and other refs (like `HEAD`) in commits when a repo is available
- Use newer protocol for release commit updating
- Strip commit SHAs for display
- Strip dotted path prefixes in release names for display

## sentry-cli 1.8.1

- Change the log format for api headers in debug logging
- Added request headers to debug logging

## sentry-cli 1.8.0

- The `info` command now returns an exit code of 1 in case the config is incomplete
- Added `--config-status-json` to the `info` command to better support sentry-cli invoked
  from scripts
- dsym batches are now calculated by size and not by file count. This should solve a few
  413 errors some users are experiencing
- The dsym upload will now skip over files that do not contain DWARF debug information
  which resolves issues where release files were uploaded as debug symbols instead of the
  actual dsym files

## sentry-cli 1.7.0

- Sourcemap uploads now automatically replace previous files with the same name.
- Honor `CLICOLOR` environment variable
- Added progress bars for source map and debug symbol upload
- No longer attempt to upload multiple versions of debug symbols with the same UUID. This
  was an issue where signed and unsigned debug symbols were discovered in derived data in
  case of debug builds.
- Support `--validate` and `--rewrite` in one command better for source map upload.

## sentry-cli 1.6.0

- Added `--fingerprint` support to `send-event`
- Added distribution support.

**Breaking Change**: releases managed for react-native and mobile are now using the new
distribution feature. Use older versions of `sentry-cli` if you do not wish to manage
distributions on self hosted Sentry versions.

## sentry-cli 1.5.0

- Added `--uuid` parameter to `upload-dsym`
- Added `--no-zips` parameter to `upload-dsym`
- Added `--derived-data` parameter to `upload-dsym`

## sentry-cli 1.4.1

- resolved an issue with some features of xcode variable expansion not working correctly

## sentry-cli 1.4.0

- Added basic support for working with the improved relases API that span projects in an
  org
- Added deploy support

## sentry-cli 1.3.0

- improved file and release list rendering
- added `sentry-cli releases propose-version`

## sentry-cli 1.2.0

- Resolved references to sourcemaps sometimes being incorrectly detected
- Resolved an issue where an incorrect Info.plist file was found (#48)
- Added support for `.env` files
- Better support SSL CA bundles on linux systems (probe more locations)
- Added `--finalize` option to automatically finalize releases on creation
- Improved `sentry-cli info` command rendering and clarity
- Added background processing for `sentry react-native-xcode`

## sentry-cli 1.1.0

- `upload-dsyms` when launched from xcode will now upload symbols in the background and
  notify with OS X notifications about changes

## sentry-cli 1.0.0

- Added support for associating dsyms with builds on supporting sentry servers

## sentry-cli 0.28.0

- Improved validation of parameters and error reporting
- Added progress bar to updater
- Added command to finalize releases

## sentry-cli 0.27.1

- Resolved an issue that the xcode integration for react native would log out a bogus
  error

## sentry-cli 0.27.0

- Added support for fetching sourcemaps from react-native's packager
- Resolved an issue with some sourcemaps not rewriting correctly

## sentry-cli 0.26.0

- Added `react-native-xcode` command to support react-native sourcemap generation and
  uploading
- Automatically create releases on sourcemap upload

## sentry-cli 0.25.0

- Resolved an issue that caused windows versions to write backslashes in URLs in release
  artifacts

## sentry-cli 0.24.0

- Fix zip upload

## sentry-cli 0.23.0

- Added support for upcoming reprocessing feature on sentry for dsym uploads.

## sentry-cli 0.22.0

- Improved dsym uploading support (fixes #29)

## sentry-cli 0.21.1

- Resolved an issue where release builds of react-native would not automatically find the
  sourcemap references

## sentry-cli 0.21.0

- Upon sourcemap uploading the `sentry-cli` tool is now automatically attempting to find
  matching sourcemaps and emit a `Sourcemap` header with the correct reference. This helps
  in situations like react-native where the source reference in the file is malformed or
  points to a non existing file by default
- fixed a bug with the `--rewrite` flag on the upload sourcemaps tool which caused
  incorrect sources to be inlined. This is now properly supported.
- `--strip-common-prefix` on the upload sourcemaps tool now skips over paths which are not
  absolute.

## sentry-cli 0.20.0

- added support for sourcemap rewriting. This will automatically inline sourcecode and
  flatten indexed sourcemaps and can optionally remove prefixes from source paths. This is
  useful for react native which otherwise will not work since sourcecode is not contained.

## sentry-cli 0.19.5

- Improved symbol uploading

## sentry-cli 0.19.4

- Improved logging of http requests
- Fixed an issue that caused a crash if the `TERM` environment variable was not set

## sentry-cli 0.19.3

- Recompiled for Linux to better support arch linux and others

## sentry-cli 0.19.2

- Resolved issue with multi-chunk dsym uploads failing

## sentry-cli 0.19.1

- Changed domain to `sentry.io`

## sentry-cli 0.19.0

- Improved handling of `SENTRY_DSN` so that it can be set to an invalid value and
  `sentry-cli` continues functioning unless you are trying to send an actual event.

## sentry-cli 0.18.0

- added the new `issues` command to bulk manage issues

## sentry-cli 0.17.0

- Added support for debug logging

## sentry-cli 0.16.1

- Upgraded the internal SHA1 library

## sentry-cli 0.16.0

- Added support for `http.proxy_url`
- Added support for `http.proxy_username`
- Added support for `http.proxy_password`

## sentry-cli 0.15.0

- Added support for the `http.keepalive` setting

## sentry-cli 0.14.0

- added proxy support
- removed global dsym uploading which is now done differently

## sentry-cli 0.13.1

- Fixed an issue that caused validation of sourcemaps to fail if wildcard paths (`~/`)
  were used.

## sentry-cli 0.13.0

- Default sourcemap url prefix to `~` to support the new wildcard feature

## sentry-cli 0.12.1

- Fixed windows support by bundling OpenSSL statically

## sentry-cli 0.12.0

- Added basic windows support
- Added `send-event` to submit events to Sentry

## sentry-cli 0.11.0

- Added `login` command.

## sentry-cli 0.10.1

- Made missing ref failures on non minimized JS files warnings instead of errors

## sentry-cli 0.10.0

- Added support for basic sourcemap validation with the `--validate` flag

## sentry-cli 0.9.0

- Ignore `--ext` for explicitly provided files on sourcemap uploads
- Properly handle `--ext`

## sentry-cli 0.8.0

- Added the ability to upload individual sourcemaps as files

## sentry-cli 0.7.0

- Added `info` command
- Addded `.sentryclirc` config file support

## sentry-cli 0.6.0

- Updated release commands

## sentry-cli 0.5.1

- Fixes uninstall support

## sentry-cli 0.5.0

Added basic sourcemap support.

## sentry-cli 0.4.0

Added sudo support to the update command.

## sentry-cli 0.3.0

Updated sentry CLI to have improved x-code dsym upload support and added an update
command.

## 0.2.0 - Alpha Release

Added support for sentry auth tokens.

## 0.1.0 - Initial Release

An initial release of the tool.
