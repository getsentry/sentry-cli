# sentry-cli

This is a Sentry command line client for some generic tasks.  Right now this
is primarily used to upload debug symbols to Sentry if you are not using the
fastlane tools.

Binaries can be found under [Releases](https://sentry.io/getsentry/sentry-cli/releases/)

You can also install it with everybody's favorite curl to bash::

    curl -sL https://getsentry.com/get-cli/ | bash

Documentation [can be found here](https://docs.sentry.io/hosted/learn/cli/)

## Compiling

In case you want to compile this yourself you need to run a nightly build of
Rust.  We keep the lock file in the repo so that builds are stable.  The
following nightly version is known to work: `2016-08-11`

Use rustup to compile:

    $ rustup override set nightly-2016-08-11
    $ cargo build

In case you get OpenSSL errors you need to compile with the path to the
OpenSSL headers.  For instance:

    $ CFLAGS=-I/usr/local/opt/openssl/include/ cargo build
