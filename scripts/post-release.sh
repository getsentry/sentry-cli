#!/bin/bash

# This script is run by Craft after a release is created.
# We currently use it to bump the platform-specific optional dependencies to their new versions
# in the package-lock.json, immediately after a release is created. This is needed for CI to
# pass after the release is created.

set -eux
OLD_VERSION="${1}"
NEW_VERSION="${2}"

git checkout master

# Craft can run this script before all platform packages for the new version are visible on npm.
# If we update the lockfile too early, npm drops their versions from it and `npm ci` fails, so
# wait until every optional dependency is available first.
OPTIONAL_DEPENDENCIES=$(node -p "Object.keys(require('./package.json').optionalDependencies).join(' ')")
WAIT_TIMEOUT_SECONDS=1800
WAIT_INTERVAL_SECONDS=30
deadline=$((SECONDS + WAIT_TIMEOUT_SECONDS))

for package in $OPTIONAL_DEPENDENCIES; do
  until [ "$(npm view "${package}@${NEW_VERSION}" version --prefer-online 2>/dev/null)" = "${NEW_VERSION}" ]; do
    if [ "${SECONDS}" -ge "${deadline}" ]; then
      echo "Timed out waiting for ${package}@${NEW_VERSION} to be available on npm" >&2
      exit 1
    fi
    sleep "${WAIT_INTERVAL_SECONDS}"
  done
done

# We need to update the package-lock.json to include the new version of the optional dependencies.
npm install --package-lock-only --ignore-scripts --prefer-online

# Don't push a lockfile that `npm ci` would reject.
NEW_VERSION="${NEW_VERSION}" node -e '
const lock = require("./package-lock.json");
const deps = Object.keys(require("./package.json").optionalDependencies);
const stale = deps.filter((name) => {
  const entry = lock.packages?.[`node_modules/${name}`] ?? lock.dependencies?.[name];
  return entry?.version !== process.env.NEW_VERSION;
});
if (stale.length > 0) {
  console.error(`package-lock.json is missing ${process.env.NEW_VERSION} for: ${stale.join(", ")}`);
  process.exit(1);
}
'

git add package-lock.json

# Only commit if there are changes
git diff --staged --quiet || git commit -m "build(npm): 🤖 Bump optional dependencies to ${NEW_VERSION}"
git pull --rebase
git push
