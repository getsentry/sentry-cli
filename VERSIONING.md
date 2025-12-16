# Versioning Policy

Sentry CLI follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html). This policy defines Sentry CLI's public API and provides examples of how we would apply this policy.

## Public API

For the purposes of Semantic Versioning, Sentry CLI's public API is defined as the following:
  - All subcommands and command line arguments.
  - All publicly exposed functions/methods of the JavaScript API (only available via the [NPM package](https://www.npmjs.com/package/@sentry/cli)).
  - The minimum self-hosted Sentry version that we support.
  - This versioning policy document.

Any of the above items, which are explicitly marked as "experimental," "beta," or similar, are not part of the public API, and are subject to change at any time.

**Anything, which is not explicitly defined as part of the public API, is not part of the public API.** In particular, for semantic versioning purposes, the following items are not part of the public API:
  - Compile-time feature flags, dependencies, MSRV, etc., as we expect most users to use the prebuilt binaries we supply.
  - Any changes to output wording/formatting/etc.
  - Any public items exported from the `sentry-cli` Rust crate, as we do not publish `sentry-cli` to crates.io, and thus do not expect anyone to use it as a library.

## Examples

This section lists examples of changes which would require a major, minor, or patch version bump. All of the lists are non-exhaustive.

### Major Version

The following changes would require a major version bump, unless the affected item is specifically marked as "experimental," "beta," or similar:
  - Removal of a subcommand or an argument to a subcommand.
  - A reduction in the accepted values that can be passed to a command line argument, unless this reduction is necessary to fix a bug, for example, because the argument never handled certain values correctly.
  - Removal of a publicly exposed function/method of the JavaScript API, or any other backwards-incompatible change to these.
  - The minimum self-hosted Sentry version supported by Sentry CLI is increased.
  - Any change to this versioning policy, which narrows the public API definition.

### Minor Version

The following changes would only require a minor version bump:
  - A new subcommand or command line argument is added, unless this new item is "experimental," "beta," or similar.
  - A new public item is added to the JavaScript API, unless this new item is "experimental," "beta," or similar.
  - An item which was previously marked "experimental," "beta," or similar has this designation removed, thus being added to the public API.
  - The minimum self-hosted Sentry version supported by Sentry CLI is decreased, i.e., we expand support to additional self-hosted versions.

### Patch Version

The following changes may occur in a patch version:
  - Bug fixes, which do not alter public API.
  - Changes to compile-time feature flags, dependencies, MSRV, etc., though we may often opt to do such changes in a minor.
  - Changes which break functionality which previously had worked with a version of self-hosted Sentry, which we do not officially support.
