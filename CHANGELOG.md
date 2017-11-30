# Changelog

## sentry-cli 1.25.0

* Do not run update nagger if the command is not connected to a terminal
* Source map uploading now correctly determines sourcemap references even
  if the rewrite flag is not passed.
* There is an offical Docker image with `sentry-cli` preinstalled:
  `docker run --rm -it -v $(pwd):/work getsentry/sentry-cli sentry-cli --help`
* Added support for automatically determining corvoda releases.

## sentry-cli 1.24.1

* Fix an issue with bash hooking not working if sentry-cli was installed on a
  path containing whitespace

## sentry-cli 1.24.0

* Improved sending events from bash. See
  [Sending Events](https://docs.sentry.io/learn/cli/send-event) for more
  information
* Hook into bash and send events for failed commands automatically. See
  [Bash Hooks](https://docs.sentry.io/learn/cli/send-event/#bash-hook) for more
  information
* Set `SENTRY_LOAD_DOTENV=0` to disable automatic loading of `.env` files
* Fix an issue where `info.plist` files were not resolved in XCode projects
* Fix an issue where the `PROJECT_DIR` environment was not used correctly

## sentry-cli 1.23.0

* Fix a bug that prevented uploads of ProGuard mapping files on Windows
* Improve command and parameter descriptions (`--help`)
* Updated dependencies

## sentry-cli 1.22.0

* Add `--ignore` and `--ignore-file` parameters to `upload-dsyms`
* Fix some typos in the CLI (thanks @mbudde and @AdrienDS)

## sentry-cli 1.21.0

* Fix codepush command for android
* Fixed added bitbucket provider support #115

## sentry-cli 1.20.0

* Updated dependencies
* Added encoding detection for javascript files
* Added bitbucket provider support
* Fixed an issue where codepush was not passing the right plist to the parser

## sentry-cli 1.19.1

* Resolved an issue where sourcemaps were not uploaded (#112)

## sentry-cli 1.19.0

* Added support for preprocessor `info.plist` files
* Unified `info.plist` handling in all places
* Added basic validation for the API URL to avoid common user errors
* Resolved an issue with NPM releases on ES5 environments
* Resolved an issue where `releases propose-version` incorrectly required an org
  to be passed
* Added support for handling `BCSymbolMap` files when uploading dsym files

## sentry-cli 1.18.0

* Ensure parent directories exist when writing Proguard meta properties.
* Write Proguard properties even if upload is disabled.
* Reject leading/trailing spaces in releases.

## sentry-cli 1.17.0

* Made npm install compatible with ES5
* Solved a potential issue with spaces in file paths for npm installation
* Added automatic update check (can be disabled with `update.disable_check` in
  the config or the `SENTRY_DISABLE_UPDATE_CHECK` environment variable)
* Resolved a crash when uploading empty files
* Lowered default symbol upload size to work around some server limitations

## sentry-cli 1.16.0

* added ability to upload proguard files with a forced UUID
* added `difutil uuid` command to print the UUID(s) of a mapping file to stdout

## sentry-cli 1.15.0

* Improved the `no-upload` flag to proguard upload
* Added debug info files debug commands

## sentry-cli 1.14.0

* Added support for disabling desktop notifications (only affects xcode builds
  so far)
* Added support for uploading proguard files on supported sentry server versions

## sentry-cli 1.13.3

* Fixed installation for npm

## sentry-cli 1.13.2

* Put `sentry-cli.exe` directly into the `bin/` folder on windows for npm
  installations

## sentry-cli 1.13.1

* Fixed another issue with yarn redownloading binaries

## sentry-cli 1.13.0

* Added `dist` support for send-event
* Improved download script for npm installs to not download unnecessarily with
  yarn.

## sentry-cli 1.12.0

* Added support for explicit bundle IDs for codepush releases
* Added `--print-release-name` to print out the release name for codepush
  releases to the command line to improve scripting capabilities
* Extended `propose-version` for releases to support iOS and android release
  names if projects are automatically discovered
* Parse grade files instead of android manifests for version and bundle IDs for
  release detection
* Fix broken xcode notifications when projects where opened from the command
  line
* Fixed limitations in automatically detecting the bundle IDs for xcode projects

## sentry-cli 1.11.1

* Resolved an issue where sourcemap uploading failed when empty files were
  encountered

## sentry-cli 1.11.0

* Initial work for codepush support (pending support in `react-native-sentry`)
* Moved `react-native-xcode` to `react-native xcode`
* Added support for `${FOO}` style variable expansion in xcode

## sentry-cli 1.10.2

* Fixed an issue for windows npm installation
* Stop generating a debug log file in `/tmp` for npm on unixes

## sentry-cli 1.10.1

* fixed a bug that caused the npm install to fail

## sentry-cli 1.10.0

* Added user support for `send-event`

## sentry-cli 1.9.2

* Improved logging facilities
* Fixed npm installation on windows

## sentry-cli 1.9.1

* Changes sourcemap rewriting to ignore bad files on source inlining.
* Fixed a bug in the JSON output of the `info` command.

## sentry-cli 1.9.0

* Added support for referring to previous hashes in `set-commits` with
  `OLD_REV..NEW_REV`
* Resolve tags and other refs (like `HEAD`) in commits when a repo is available
* Use newer protocol for release commit updating
* Strip commit SHAs for display
* Strip dotted path prefixes in release names for display

## sentry-cli 1.8.1

* Change the log format for api headers in debug logging
* Added request headers to debug logging

## sentry-cli 1.8.0

* The `info` command now returns an exit code of 1 in case the config is
  incomplete
* Added `--config-status-json` to the `info` command to better support
  sentry-cli invoked from scripts
* dsym batches are now calculated by size and not by file count. This should
  solve a few 413 errors some users are experiencing
* The dsym upload will now skip over files that do not contain DWARF debug
  information which resolves issues where release files were uploaded as debug
  symbols instead of the actual dsym files

## sentry-cli 1.7.0

* Sourcemap uploads now automatically replace previous files with the same name.
* Honor `CLICOLOR` environment variable
* Added progress bars for source map and debug symbol upload
* No longer attempt to upload multiple versions of debug symbols with the same
  UUID. This was an issue where signed and unsigned debug symbols were
  discovered in derived data in case of debug builds.
* Support `--validate` and `--rewrite` in one command better for source map
  upload.

## sentry-cli 1.6.0

* Added `--fingerprint` support to `send-event`
* Added distribution support.

**Breaking Change**: releases managed for react-native and mobile are now using
the new distribution feature. Use older versions of `sentry-cli` if you do not
wish to manage distributions on self hosted Sentry versions.

## sentry-cli 1.5.0

* Added `--uuid` parameter to `upload-dsym`
* Added `--no-zips` parameter to `upload-dsym`
* Added `--derived-data` parameter to `upload-dsym`

## sentry-cli 1.4.1

* resolved an issue with some features of xcode variable expansion not working
  correctly

## sentry-cli 1.4.0

* Added basic support for working with the improved relases API that span
  projects in an org
* Added deploy support

## sentry-cli 1.3.0

* improved file and release list rendering
* added `sentry-cli releases propose-version`

## sentry-cli 1.2.0

* Resolved references to sourcemaps sometimes being incorrectly detected
* Resolved an issue where an incorrect Info.plist file was found (#48)
* Added support for `.env` files
* Better support SSL CA bundles on linux systems (probe more locations)
* Added `--finalize` option to automatically finalize releases on creation
* Improved `sentry-cli info` command rendering and clarity
* Added background processing for `sentry react-native-xcode`

## sentry-cli 1.1.0

* `upload-dsyms` when launched from xcode will now upload symbols in the
  background and notify with OS X notifications about changes

## sentry-cli 1.0.0

* Added support for associating dsyms with builds on supporting sentry servers

## sentry-cli 0.28.0

* Improved validation of parameters and error reporting
* Added progress bar to updater
* Added command to finalize releases

## sentry-cli 0.27.1

* Resolved an issue that the xcode integration for react native would log out a
  bogus error

## sentry-cli 0.27.0

* Added support for fetching sourcemaps from react-native's packager
* Resolved an issue with some sourcemaps not rewriting correctly

## sentry-cli 0.26.0

* Added `react-native-xcode` command to support react-native sourcemap
  generation and uploading
* Automatically create releases on sourcemap upload

## sentry-cli 0.25.0

* Resolved an issue that caused windows versions to write backslashes in URLs in
  release artifacts

## sentry-cli 0.24.0

* Fix zip upload

## sentry-cli 0.23.0

* Added support for upcoming reprocessing feature on sentry for dsym uploads.

## sentry-cli 0.22.0

* Improved dsym uploading support (fixes #29)

## sentry-cli 0.21.1

* Resolved an issue where release builds of react-native would not automatically
  find the sourcemap references

## sentry-cli 0.21.0

* Upon sourcemap uploading the `sentry-cli` tool is now automatically attempting
  to find matching sourcemaps and emit a `Sourcemap` header with the correct
  reference. This helps in situations like react-native where the source
  reference in the file is malformed or points to a non existing file by default
* fixed a bug with the `--rewrite` flag on the upload sourcemaps tool which
  caused incorrect sources to be inlined. This is now properly supported.
* `--strip-common-prefix` on the upload sourcemaps tool now skips over paths
  which are not absolute.

## sentry-cli 0.20.0

* added support for sourcemap rewriting. This will automatically inline
  sourcecode and flatten indexed sourcemaps and can optionally remove prefixes
  from source paths. This is useful for react native which otherwise will not
  work since sourcecode is not contained.

## sentry-cli 0.19.5

* Improved symbol uploading

## sentry-cli 0.19.4

* Improved logging of http requests
* Fixed an issue that caused a crash if the `TERM` environment variable was not
  set

## sentry-cli 0.19.3

* Recompiled for Linux to better support arch linux and others

## sentry-cli 0.19.2

* Resolved issue with multi-chunk dsym uploads failing

## sentry-cli 0.19.1

* Changed domain to `sentry.io`

## sentry-cli 0.19.0

* Improved handling of `SENTRY_DSN` so that it can be set to an invalid value
  and `sentry-cli` continues functioning unless you are trying to send an actual
  event.

## sentry-cli 0.18.0

* added the new `issues` command to bulk manage issues

## sentry-cli 0.17.0

* Added support for debug logging

## sentry-cli 0.16.1

* Upgraded the internal SHA1 library

## sentry-cli 0.16.0

* Added support for `http.proxy_url`
* Added support for `http.proxy_username`
* Added support for `http.proxy_password`

## sentry-cli 0.15.0

* Added support for the `http.keepalive` setting

## sentry-cli 0.14.0

* added proxy support
* removed global dsym uploading which is now done differently

## sentry-cli 0.13.1

* Fixed an issue that caused validation of sourcemaps to fail if wildcard paths
  (`~/`) were used.

## sentry-cli 0.13.0

* Default sourcemap url prefix to `~` to support the new wildcard feature

## sentry-cli 0.12.1

* Fixed windows support by bundling OpenSSL statically

## sentry-cli 0.12.0

* Added basic windows support
* Added `send-event` to submit events to Sentry

## sentry-cli 0.11.0

* Added `login` command.

## sentry-cli 0.10.1

* Made missing ref failures on non minimized JS files warnings instead of errors

## sentry-cli 0.10.0

* Added support for basic sourcemap validation with the `--validate` flag

## sentry-cli 0.9.0

* Ignore `--ext` for explicitly provided files on sourcemap uploads
* Properly handle `--ext`

## sentry-cli 0.8.0

* Added the ability to upload individual sourcemaps as files

## sentry-cli 0.7.0

* Added `info` command
* Addded `.sentryclirc` config file support

## sentry-cli 0.6.0

* Updated release commands

## sentry-cli 0.5.1

* Fixes uninstall support

## sentry-cli 0.5.0

Added basic sourcemap support.

## sentry-cli 0.4.0

Added sudo support to the update command.

## sentry-cli 0.3.0

Updated sentry CLI to have improved x-code dsym upload support and added an
update command.

## 0.2.0 - Alpha Release

Added support for sentry auth tokens.

## 0.1.0 - Initial Release

An initial release of the tool.
