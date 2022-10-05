<p align="center">
  <a href="https://sentry.io/?utm_source=github&utm_medium=logo" target="_blank">
    <picture>
      <source srcset="https://sentry-brand.storage.googleapis.com/sentry-logo-white.png" media="(prefers-color-scheme: dark)" />
      <source srcset="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" media="(prefers-color-scheme: light), (prefers-color-scheme: no-preference)" />
      <img src="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" alt="Sentry" width="280">
    </picture>
  </a>
</p>

# Official Sentry Command Line Interface

[![Build Status](https://github.com/getsentry/sentry-cli/workflows/CI/badge.svg?branch=master)](https://github.com/getsentry/sentry-cli/actions?query=workflow%3ACI)
[![GitHub release](https://img.shields.io/github/release/getsentry/sentry-cli.svg)](https://github.com/getsentry/sentry-cli/releases/latest)
[![npm version](https://img.shields.io/npm/v/@sentry/cli.svg)](https://www.npmjs.com/package/@sentry/cli)
[![license](https://img.shields.io/github/license/getsentry/sentry-cli.svg)](https://github.com/getsentry/sentry-cli/blob/master/LICENSE)

This is a Sentry command line client for some generic tasks. Right now this is
primarily used to upload debug symbols to Sentry if you are not using the
fastlane tools.

* Downloads can be found under
  [Releases](https://github.com/getsentry/sentry-cli/releases/)
* Documentation can be found [here](https://docs.sentry.io/hosted/learn/cli/)

## Installation

If you are on OS X or Linux, you can use the automated downloader which will fetch the latest release version for you and install it:

    curl -sL https://sentry.io/get-cli/ | bash

We do however, encourage you to pin the specific version of the CLI, so your builds are always reproducible.
To do that, you can use the exact same method, with an additional version specifier:

    curl -sL https://sentry.io/get-cli/ | SENTRY_CLI_VERSION=2.0.4 bash

This will automatically download the correct version of `sentry-cli` for your operating system and install it. If necessary, it will prompt for your admin password for `sudo`. For a different installation location or for systems without `sudo` (like Windows), you can `export INSTALL_DIR=/custom/installation/path` before running this command.

If you are using `sentry-cli` on Windows environments, [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) is required.

To verify it’s installed correctly you can bring up the help:

    sentry-cli --help

### Node

The `sentry-cli` binary can also be installed as part of the `@sentry/cli` npm package. See the [`@sentry/cli` section below](#sentrycli) for details.


### Homebrew

A homebrew recipe is provided in the `getsentry/tools` tap:

    brew install getsentry/tools/sentry-cli

### Docker

As of version _1.25.0_, there is an official Docker image that comes with
`sentry-cli` preinstalled. If you prefer a specific version, specify it as tag.
The latest development version is published under the `edge` tag. In production,
we recommend you to use the `latest` tag. To use it, run:

```sh
docker pull getsentry/sentry-cli
docker run --rm -v $(pwd):/work getsentry/sentry-cli --help
```

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

## `@sentry/cli`

The `sentry-cli` binary is also available as part of the `@sentry/cli` npm package.

### Installation

```sh
npm install @sentry/cli
```

This will install the npm package, which will then download the appropriate binary for your operating system. (To skip downloading and use a local copy of the binary when using `@sentry/cli`, make sure it's in your PATH and set `SENTRYCLI_USE_LOCAL=1` in your environment.)

If installing globally, make sure to have set [correct permissions on the global node_modules directory](https://docs.npmjs.com/getting-started/fixing-npm-permissions). If this is not possible in your environment or still produces an EACCESS error, install as root:

```sh
sudo npm install -g @sentry/cli --unsafe-perm
```

If you're installing the CLI with NPM from behind a proxy, the install script will use either NPM's configured HTTPS proxy server, or the value from your `HTTPS_PROXY` environment variable.

### Changing Download CDN

By default, this package will download sentry-cli from the CDN managed by [Fastly](https://www.fastly.com/).
To use a different CDN, either an established one like GitHub (`https://github.com/getsentry/sentry-cli/releases/download/`) or a custom one (for example `http://www.some.cdn/some/path/`), set it as the CDN URL when installing `@sentry/cli`, using one of the following methods:

- Using a CLI flag
  ```sh
  npm install @sentry/cli --sentrycli_cdnurl=http://www.some.cdn/some/path
  ```
- Adding it as a property in your [`.npmrc` file](https://www.npmjs.org/doc/files/npmrc.html)
  ```rc
  sentrycli_cdnurl=http://www.some.cdn/some/path
  ```
- Using an environment variable
  ```sh
  SENTRYCLI_CDNURL=http://www.some.cdn/some/path npm install @sentry/cli
  ```

If using a custom CDN like `http://www.some.cdn/some/path/`, perform the following on the machine hosting `sentry-cli` (the one reachable at `http://www.some.cdn`):

1. Install `sentry-cli` using any of the methods listed in this README.
2. Run `cd some/path && node customCDNHelper.js`. (`customCDNHelper.js` can be found [here](https://github.com/getsentry/sentry-cli/blob/master/scripts/customCDNHelper.js) in the `scripts` directory in this repo.) This will move and rename the binary so `@sentry/cli` can find it when downloading.

Make sure the version being hosted matches the version listed in `@sentry/cli`'s `package.json`.
