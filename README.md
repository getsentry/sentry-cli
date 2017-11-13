<p align="center">
    <img src="https://sentry-brand.storage.googleapis.com/sentry-logo-black.png" width="280">
    <br />
</p>

# Official Sentry Command Line Interface

[![Travis](https://img.shields.io/travis/getsentry/sentry-cli.svg)](https://travis-ci.org/getsentry/sentry-cli)
[![AppVeyor](https://img.shields.io/appveyor/ci/sentry/sentry-cli.svg)](https://ci.appveyor.com/project/sentry/sentry-cli)
[![GitHub release](https://img.shields.io/github/release/getsentry/sentry-cli.svg)](github.com/getsentry/sentry-cli/releases/latest)
[![npm version](https://img.shields.io/npm/v/sentry-cli-binary.svg)](https://www.npmjs.com/package/sentry-cli-binary)
[![license](https://img.shields.io/github/license/getsentry/sentry-cli.svg)](https://github.com/getsentry/sentry-cli/blob/master/LICENSE)

This is a Sentry command line client for some generic tasks.  Right now this
is primarily used to upload debug symbols to Sentry if you are not using the
fastlane tools.

 - Binaries can be found under [Releases](https://github.com/getsentry/sentry-cli/releases/)
 - Documentation can be found [here](https://docs.sentry.io/hosted/learn/cli/)

## Installation

The recommended way to install is with everybody's favorite curl to bash:

    curl -sL https://sentry.io/get-cli/ | bash

Additionally you can also install this binary via npm:

    npm install sentry-cli-binary

When installing globally, make sure to have set [correct permissions on the global node_modules directory](https://docs.npmjs.com/getting-started/fixing-npm-permissions).
If this is not possible in your environment or still produces an EACCESS error, install as root:

    sudo npm install -g sentry-cli-binary --unsafe-perm

Or homebrew:

    brew install getsentry/tools/sentry-cli

## Compiling

In case you want to compile this yourself you need to build this with Rust
1.15 or later.

Use rustup to compile:

    $ cargo build

In case you get OpenSSL errors you need to compile with the path to the
OpenSSL headers.  For instance:

    $ CFLAGS=-I/usr/local/opt/openssl/include/ cargo build
