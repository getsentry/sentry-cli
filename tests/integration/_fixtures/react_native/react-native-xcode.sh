#!/bin/sh

echo "react-native-xcode.sh called with args: $@"

echo '{
  "packager_bundle_path": "tests/integration/_fixtures/react_native/react-native-xcode-bundle.js",
  "packager_sourcemap_path": "tests/integration/_fixtures/react_native/react-native-xcode-bundle.js.map"
}' > "$SENTRY_RN_SOURCEMAP_REPORT"
