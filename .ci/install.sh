#!/bin/bash
set -ex
curl https://static.rust-lang.org/rustup.sh | sh -s -- --with-target=$TARGET
