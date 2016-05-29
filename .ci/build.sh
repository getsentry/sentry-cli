#!/bin/bash
set -ex

rustup target add $TARGET || true
cargo build --target $TARGET --release
