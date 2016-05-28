#!/bin/bash
set -ex
curl https://sh.rustup.rs -sSf | sh -s -- -y
rustup override set nightly-2016-05-01
rustc --version
cargo --version
