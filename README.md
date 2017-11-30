<p align="center">
    <img src="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" width="280">
    <br />
</p>

# Official Sentry Command Line Interface

[![Travis](https://img.shields.io/travis/getsentry/sentry-cli.svg)](https://travis-ci.org/getsentry/sentry-cli)
[![AppVeyor](https://img.shields.io/appveyor/ci/sentry/sentry-cli.svg)](https://ci.appveyor.com/project/sentry/sentry-cli)
[![GitHub release](https://img.shields.io/github/release/getsentry/sentry-cli.svg)](https://github.com/getsentry/sentry-cli/releases/latest)
[![npm version](https://img.shields.io/npm/v/sentry-cli-binary.svg)](https://www.npmjs.com/package/sentry-cli-binary)
[![license](https://img.shields.io/github/license/getsentry/sentry-cli.svg)](https://github.com/getsentry/sentry-cli/blob/master/LICENSE)

This is a Sentry command line client for some generic tasks. Right now this is
primarily used to upload debug symbols to Sentry if you are not using the
fastlane tools.

* Binaries can be found under
  [Releases](https://github.com/getsentry/sentry-cli/releases/)
* Documentation can be found [here](https://docs.sentry.io/hosted/learn/cli/)

## Installation

The recommended way to install is with everybody's favorite curl to bash:

    curl -sL https://sentry.io/get-cli/ | bash

Additionally you can also install this binary via npm:

    npm install sentry-cli-binary

When installing globally, make sure to have set
[correct permissions on the global node_modules directory](https://docs.npmjs.com/getting-started/fixing-npm-permissions).
If this is not possible in your environment or still produces an EACCESS error,
install as root:

    sudo npm install -g sentry-cli-binary --unsafe-perm

Or homebrew:

    brew install getsentry/tools/sentry-cli

As of version _1.25.0_, there is an official Docker image that comes with
`sentry-cli` preinstalled. If you prefer a specific version, specify it as tag.
The latest development version is published under the `edge` tag. In production,
we recommend you to use the `latest` tag. To use it, run:

```sh
docker pull getsentry/sentry-cli
docker run --rm -it -v $(pwd):/work getsentry/sentry-cli sentry-cli --help
```

## Compiling

In case you want to compile this yourself, you need to install at minimum the
following dependencies:

* C and C++ 11 compiler
* Make and CMake
* OpenSSL 1.0.2j with development headers
* Curl 7.50 with development headers
* Rust 1.20 and Cargo

Use cargo to compile:

    $ cargo build

In case you get OpenSSL errors you need to compile with the path to the OpenSSL
headers. For instance:

    $ CFLAGS=-I/usr/local/opt/openssl/include/ cargo build

Also, there is a Dockerfile that builds an Alpine-based Docker image with
`sentry-cli` in the PATH. Note that this image is not minimal yet, as we are not
able to compail against musl just yet. To build and use it, run:

```sh
docker build -t sentry-cli .
docker run --rm -it -v $(pwd):/work sentry-cli sentry-cli --help
```
