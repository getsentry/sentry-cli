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

# Bump main sentry cli npm package
npm version "${TARGET}"

# Bump the binary npm distributions
for dir in $SCRIPT_DIR/../npm-binary-distributions/*; do
    cd $dir
    npm version "${TARGET}"
    cd -
done

# Update the optional deps in the main cli npm package
# Requires jq to be installed - should be installed ootb on github runners
jq '.optionalDependencies |= map_values("'"${TARGET}"'")' $SCRIPT_DIR/../package.json > package.json.tmp && mv package.json.tmp $SCRIPT_DIR/../package.json

# Pin the optional deps in package-lock.json too, so `npm ci` accepts the lockfile as soon as
# this commit lands. The platform packages aren't on npm yet, so we can't let npm resolve them;
# entries with only a version are enough, and `npm ci` skips optional deps it can't fetch.
if [ "$(jq '.lockfileVersion' $SCRIPT_DIR/../package-lock.json)" != "1" ]; then
    echo "package-lock.json is no longer lockfileVersion 1; update this script to match." >&2
    exit 1
fi
jq --arg version "${TARGET}" --slurpfile pkg $SCRIPT_DIR/../package.json \
    'reduce ($pkg[0].optionalDependencies | keys[]) as $name (.; .dependencies[$name] = {version: $version, optional: true})' \
    $SCRIPT_DIR/../package-lock.json > package-lock.json.tmp && mv package-lock.json.tmp $SCRIPT_DIR/../package-lock.json
