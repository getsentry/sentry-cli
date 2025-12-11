<p align="center">
  <a href="https://sentry.io/?utm_source=github&utm_medium=logo" target="_blank">
    <picture>
      <source srcset="https://sentry-brand.storage.googleapis.com/sentry-logo-white.png" media="(prefers-color-scheme: dark)" />
      <source srcset="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" media="(prefers-color-scheme: light), (prefers-color-scheme: no-preference)" />
      <img src="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" alt="Sentry" width="280">
    </picture>
  </a>
</p>

# Sentry CLI

This is the repository for Sentry CLI, the official command line interface for Sentry.

Sentry CLI can be used for many tasks, including uploading debug symbols and source maps to Sentry, managing releases, and viewing Sentry data such as issues and logs.

## Installation and Usage

Please refer to [Sentry CLI's documentation page](https://docs.sentry.io/cli/).

## Compatibility

Sentry CLI officially supports [Sentry SaaS](https://sentry.io/) and [Sentry Self-Hosted](https://github.com/getsentry/self-hosted) versions 25.11.1 and above.

### Self-Hosted Sentry

Although some Sentry CLI features may work with versions of Sentry Self-Hosted prior to 25.11.1, we recommend users upgrade their self-hosted installations to a compatible version.

For users who cannot upgrade their self-hosted installation, we recommend using the latest compatible Sentry CLI version, per the table below:

| **Sentry Self-Hosted Version** | **Newest Compatible Sentry CLI Version**                              |
| ------------------------------ | --------------------------------------------------------------------- |
| ≥ 25.11.1                      | [latest](https://github.com/getsentry/sentry-cli/releases/latest)     |
| < 25.11.1                      | [2.58.4](https://github.com/getsentry/sentry-cli/releases/tag/2.58.4) |

Note that we can only provide support for officially-supported Sentry Self-Hosted versions. We will not backport fixes for older Sentry CLI versions, even if they should be compatible with your self-hosted version.

## Compiling

In case you want to compile this yourself, you need to install at minimum the
following dependencies:

* Rust stable and Cargo
* Make, CMake and a C compiler

Use cargo to compile:

    $ cargo build

Also, there is a Dockerfile that builds an Alpine-based Docker image with
`sentry-cli` in the PATH. To build and use it, run:

```sh
docker build -t sentry-cli .
docker run --rm -v $(pwd):/work sentry-cli --help
```
