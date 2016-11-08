#!/bin/bash
set -ex

rustc --version
cargo --version

rustup target add $TARGET || true
cargo build --target $TARGET --release
