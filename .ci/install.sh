#!/bin/bash
set -ex
curl https://sh.rustup.rs -sSf | sh -s -- -y
rustup override set nightly-2016-05-22
rustc --version
cargo --version

ls -alh /usr/lib/gcc/*
