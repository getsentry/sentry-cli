#!/bin/bash
set -ex
curl https://sh.rustup.rs -sSf | sh -s -- -y
rustup override set nightly-2016-10-08
rustc --version
cargo --version
