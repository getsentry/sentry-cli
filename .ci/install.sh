#!/bin/bash
set -ex
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain=nightly-2016-05-01
rustc --version
cargo --version
