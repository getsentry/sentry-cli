#!/bin/bash
set -ex

rustup target add $TARGET || true
cargo build --target $TARGET --release

if [ $TARGET != "i686-unknown-linux-gnu" ]; then
  cargo test --release
fi
