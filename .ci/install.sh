#!/bin/bash
set -ex
curl https://static.rust-lang.org/rustup.sh | sh -s -- --prefix=$HOME/rust --spec=nightly-2016-10-08 --with-target=$TARGET
