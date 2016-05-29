#!/bin/bash
set -ex

export OPENSSL_STATIC=1

rustup target add $TARGET || true
cargo build --target $TARGET --release
