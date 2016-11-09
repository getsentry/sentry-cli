#!/bin/bash
set -ex

rustc --version
cargo --version

cargo build --release --target=$TARGET
