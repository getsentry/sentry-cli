#!/bin/bash
set -ex
curl https://static.rust-lang.org/rustup.sh | sh -s -- --prefix=$HOME/rust --with-target=$TARGET
