#!/bin/bash
set -eux

if [ "$(uname -s)" != "Linux" ]; then
    echo "sentry-cli can only be released on Linux!"
    echo "Please use the GitHub Action instead."
    exit 1
fi

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPT_DIR/..

VERSION="${1}"
TARGET="${2}"

echo "Current version: $VERSION"
echo "Bumping version: $TARGET"

perl -pi -e "s/^version = .*\$/version = \"$TARGET\"/" Cargo.toml
cargo update -p sentry-cli

# Do not tag and commit changes made by "npm version"
export npm_config_git_tag_version=false
npm version "${TARGET}"
