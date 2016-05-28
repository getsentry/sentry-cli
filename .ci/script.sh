#!/bin/bash
set -ex

rustup target add $TARGET
cargo build --target $TARGET --release
