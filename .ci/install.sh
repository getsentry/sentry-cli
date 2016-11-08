#!/bin/bash
set -ex
curl https://sh.rustup.rs -sSf | sh -s -- -y --prefix=$HOME/rust --spec=nightly-2016-10-08 --with-target=$TARGET
rustc --version
cargo --version
