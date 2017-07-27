# sentry-cli

This is a Sentry command line client for some generic tasks.  Right now this
is primarily used to upload debug symbols to Sentry if you are not using the
fastlane tools.

Binaries can be found under [Releases](https://github.com/getsentry/sentry-cli/releases/)

You can also install it with everybody's favorite curl to bash:

    curl -sL https://sentry.io/get-cli/ | bash

Additionally you can also install this binary via npm:

    npm install sentry-cli-binary

Or homebrew:

    brew install getsentry/tools/sentry-cli

Documentation [can be found here](https://docs.sentry.io/hosted/learn/cli/)

## Compiling

In case you want to compile this yourself you need to build this with Rust
1.15 or later.

Use rustup to compile:

    $ cargo build

In case you get OpenSSL errors you need to compile with the path to the
OpenSSL headers.  For instance:

    $ CFLAGS=-I/usr/local/opt/openssl/include/ cargo build
