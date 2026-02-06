# Sentry CLI Feature Snapshot

> [!NOTE]
> This document was updated on **2026-02-06** and is accurate as of that date. We do not intend to actively maintain this document; it should only be considered as a snapshot of the Sentry CLI feature surface area at that time.

## Scope

- Source: `sentry-cli help` output collected via `cargo run`.
- CLI version: `sentry-cli 3.1.0`.

## Authentication And Configuration

Commands that authenticate with Sentry and report the current configuration/auth status.

- `login` — Authenticate with the Sentry server.
- `info` — Print information about the configuration and verify authentication.

## Organization And Project Administration

Commands for managing organizational structures, projects, and repository integrations.

- `organizations` — Manage organizations on Sentry.
- `organizations list` — List all organizations available to the authenticated token.
- `projects` — Manage projects on Sentry.
- `projects list` — List all projects for an organization.
- `repos` — Manage repositories on Sentry.
- `repos list` — List all repositories in your organization.

## Releases And Deploys

Commands that create, manage, and annotate releases and deployments.

- `releases` — Manage releases on Sentry.
- `releases archive` — Archive a release.
- `releases delete` — Delete a release.
- `releases finalize` — Mark a release as finalized and released.
- `releases info` — Print information about a release.
- `releases list` — List the most recent releases.
- `releases new` — Create a new release.
- `releases propose-version` — Propose a version name for a new release.
- `releases restore` — Restore a release.
- `releases set-commits` — Set commits of a release.
- `deploys` — Manage deployments for Sentry releases.
- `deploys list` — List all deployments of a release.
- `deploys new` — Creates a new release deployment.
- `build` — Manage builds.
- `build upload` — Upload builds to a project.

## Artifacts And Symbolication

Commands that upload, analyze, or manage build artifacts and symbolication inputs.

- `sourcemaps` — Manage sourcemaps for Sentry releases.
- `sourcemaps inject` — Fixes up JavaScript source files and sourcemaps with debug ids.
- `sourcemaps resolve` — Resolve sourcemap for a given line/column position.
- `sourcemaps upload` — Upload sourcemaps for a release.
- `debug-files` — Locate, analyze or upload debug information files.
- `debug-files bundle-sources` — Create a source bundle for a given debug information file
- `debug-files check` — Check the debug info file at a given path.
- `debug-files find` — Locate debug information files for given debug identifiers.
- `debug-files print-sources` — Print source files linked by the given debug info file.
- `debug-files upload` — Upload debugging information files.
- `dif` — Alias for `debug-files` (includes all subcommands).
- `upload-proguard` — Upload ProGuard mapping files to a project.
- `dart-symbol-map` — Manage Dart/Flutter symbol maps for Sentry.
- `dart-symbol-map upload` — Upload a Dart/Flutter symbol map (dartsymbolmap) for deobfuscating Dart exception types.
- `react-native` — Upload build artifacts for react-native projects.
- `react-native gradle` — Upload react-native projects in a gradle build step.
- `react-native xcode` — Upload react-native projects in a Xcode build step.

## Events, Issues, And Logs

Commands for inspecting or sending events, managing issues, and working with logs.

- `events` — Manage events on Sentry.
- `events list` — List all events in your organization.
- `issues` — Manage issues in Sentry.
- `issues list` — List all issues in your organization.
- `issues mute` — Bulk mute all selected issues.
- `issues resolve` — Bulk resolve all selected issues.
- `issues unresolve` — Bulk unresolve all selected issues.
- `send-event` — Send a manual event to Sentry.
- `send-envelope` — Send a stored envelope to Sentry.
- `logs` — [BETA] Manage logs in Sentry
- `logs list` — [BETA] List logs from your organization

## Monitoring

Commands for managing cron monitors and related schedules.

- `monitors` — Manage cron monitors on Sentry.
- `monitors list` — List all monitors for an organization.
- `monitors run` — Wraps a command

## Utilities And Maintenance

Commands for local tooling, shell integrations, and uninstalling the CLI.

- `completions` — Generate completions for the specified shell.
- `uninstall` — Uninstall the sentry-cli executable (may be hidden depending on installation method).
- `update` — Update the sentry-cli executable (may be hidden depending on installation method).

## Hidden Commands

Commands that are hidden from standard help output, kept for backward compatibility or internal tooling.

- `bash-hook` — Emit a bash helper script for error handling and (optionally) send a failure event.
- `debug-files bundle-jvm` — Experimental command to create JVM source bundles.
- `difutil` — Hidden alias for `debug-files`.
- `debug-files id` — Hidden alias for `debug-files check`.
- `debug-files uuid` — Hidden alias for `debug-files check`.
- `releases deploys` — Legacy `releases deploys <VERSION>` compatibility wrapper.
- `upload-dif` — Hidden alias for `debug-files upload`.
- `upload-dsym` — Hidden alias for `debug-files upload`.

## Appendix: Command Tree

Note: entries in parentheses (like `(bundle-jvm)`) indicate hidden commands or hidden aliases. Entries in square brackets (like `[update]`) indicate commands that may be hidden depending on installation method.

```text
sentry-cli
├── completions
├── build
│   └── upload
├── debug-files
│   ├── bundle-sources
│   ├── check
│   ├── find
│   ├── print-sources
│   ├── upload
│   ├── (bundle-jvm)
│   ├── (id)
│   └── (uuid)
├── dif
│   ├── bundle-sources
│   ├── check
│   ├── find
│   ├── print-sources
│   ├── upload
│   ├── (bundle-jvm)
│   ├── (id)
│   └── (uuid)
├── deploys
│   ├── list
│   └── new
├── events
│   └── list
├── info
├── issues
│   ├── list
│   ├── mute
│   ├── resolve
│   └── unresolve
├── login
├── logs
│   └── list
├── monitors
│   ├── list
│   └── run
├── organizations
│   └── list
├── projects
│   └── list
├── react-native
│   ├── gradle
│   └── xcode
├── releases
│   ├── archive
│   ├── delete
│   ├── finalize
│   ├── info
│   ├── list
│   ├── new
│   ├── propose-version
│   ├── restore
│   ├── set-commits
│   └── (deploys)
│       ├── list
│       └── new
├── repos
│   └── list
├── send-event
├── send-envelope
├── sourcemaps
│   ├── inject
│   ├── resolve
│   └── upload
├── dart-symbol-map
│   └── upload
├── upload-proguard
├── [uninstall]
├── [update]
├── (bash-hook)
├── (difutil)
│   ├── bundle-sources
│   ├── check
│   ├── find
│   ├── print-sources
│   ├── upload
│   ├── (bundle-jvm)
│   ├── (id)
│   └── (uuid)
├── (upload-dif)
├── (upload-dsym)
└── help
```
