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

Sentry CLI officially supports [Sentry SaaS](https://sentry.io/) and [Sentry Self-Hosted](https://github.com/getsentry/self-hosted) versions 24.11.1 and above.

<details>

<summary><h3>Self-Hosted Sentry</h3></summary>

For self-hosted installations, only those features which were available in Sentry CLI at the time of the release of the given self-hosted version are supported, as new features may require server-side support. Additionally, some features, like the `sentry-cli build` commands, are restricted to Sentry SaaS.

Users who are using Sentry Self-Hosted versions older than 24.11.1 are encouraged to upgrade their Sentry Self-Hosted installations before using Sentry CLI versions 3.0.0 and above. For users who cannot upgrade, please use the version indicated in the table below.

| **Sentry Self-Hosted Version** | **Newest Compatible Sentry CLI Version**                              |
| ------------------------------ | --------------------------------------------------------------------- |
| ≥ 24.11.1                      | [latest](https://github.com/getsentry/sentry-cli/releases/latest)     |
| < 24.11.1                      | [2.58.4](https://github.com/getsentry/sentry-cli/releases/tag/2.58.4) |

Note that we can only provide support for officially-supported Sentry Self-Hosted versions. We will not backport fixes for older Sentry CLI versions, even if they should be compatible with your self-hosted version.

</details>

## Versioning

Sentry CLI follows semantic versioning, according to [this versioning policy](VERSIONING.md).

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
