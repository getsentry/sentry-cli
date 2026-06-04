# Sentry CLI consumers in `getsentry`

## Method

This list is based on public GitHub code search for `@sentry/cli`, targeted checks for `sentry-cli` binary usage, and the consumers found while preparing the 2.x and 3.x update plans. Links use commit-pinned GitHub URLs.

Excluded: documentation-only examples and changelog mentions unless the file installs or declares Sentry CLI.

## Direct package, installer, or binary consumers

| Repo | Location | How Sentry CLI is pulled in |
| --- | --- | --- |
| `getsentry/action-release` | [`package.json`](https://github.com/getsentry/action-release/blob/6057f7e45acda771603cde6e19e0fe30310566e8/package.json#L32) | npm dependency: `"@sentry/cli": "^2.58.4"` |
| `getsentry/action-release` | [`action.yml`](https://github.com/getsentry/action-release/blob/6057f7e45acda771603cde6e19e0fe30310566e8/action.yml#L205) | runtime install: `npm install --no-package-lock @sentry/cli@^2.4` |
| `getsentry/eng-pipes` | [`package.json`](https://github.com/getsentry/eng-pipes/blob/32ffc46499bbb7c9d2db2235fc5b6ed34455d33e/package.json#L74) | dev dependency: `"@sentry/cli": "^2.20.1"` |
| `getsentry/plausible-mcp` | [`package.json`](https://github.com/getsentry/plausible-mcp/blob/2e0b3749a62cbdc673a178e809c2d828236b68f0/package.json#L36) | dev dependency: `"@sentry/cli": "^2"` |
| `getsentry/sentry-capacitor` | [`example/ionic-vue3/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-vue3/package.json#L31) | dependency: `"@sentry/cli": "^2.58.4"` |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v7/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-angular-v7/package.json#L32) | dev dependency: `"@sentry/cli": "^2.21.2"` |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v8/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-angular-v8/package.json#L32) | dev dependency: `"@sentry/cli": "^2.21.2"` |
| `getsentry/sentry-javascript` | [`packages/remix/package.json`](https://github.com/getsentry/sentry-javascript/blob/0a8adc4f67dcb4eedbc4a5454dad49a0d9d5305d/packages/remix/package.json#L71) | dependency: `"@sentry/cli": "^2.58.6"` |
| `getsentry/sentry-javascript` | [`packages/react-router/package.json`](https://github.com/getsentry/sentry-javascript/blob/0a8adc4f67dcb4eedbc4a5454dad49a0d9d5305d/packages/react-router/package.json#L53) | dependency: `"@sentry/cli": "^2.58.6"` |
| `getsentry/sentry-javascript-bundler-plugins` | [`packages/bundler-plugin-core/package.json`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/ac484d82fd9f3259d58acc421b4971dd4e5b46ce/packages/bundler-plugin-core/package.json#L57) | dependency: `"@sentry/cli": "^2.58.5"` |
| `getsentry/sentry-react-native` | [`package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/package.json#L33) | root dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-react-native` | [`packages/core/package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/packages/core/package.json#L78) | package dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-react-native` | [`packages/expo-upload-sourcemaps/package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/packages/expo-upload-sourcemaps/package.json#L30) | package dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-react-native` | [`.github/workflows/e2e-v2.yml`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/.github/workflows/e2e-v2.yml#L123) | CI install: `npm i -g react-native-cli @sentry/cli` |
| `getsentry/wif` | [`package.json`](https://github.com/getsentry/wif/blob/c176c82f0bfbb1f9a9d8168870b5cbd645162d13/package.json#L46) | dev dependency: `"@sentry/cli": "^3.2.0"` |
| `getsentry/sentry-dart-plugin` | [`lib/src/cli/_sources.dart`](https://github.com/getsentry/sentry-dart-plugin/blob/cafaabeb46b96991dd2899c5f8fb127ec0083815/lib/src/cli/_sources.dart#L7) | hardcoded Sentry CLI binary source version: `2.52.0` |

## Lockfile-only or transitive consumers

These repos have Sentry CLI in lockfiles or package-manager build-approval config, but no direct `@sentry/cli` dependency declaration was found in the searched manifest files.

| Repo | Location | Locked or configured Sentry CLI reference |
| --- | --- | --- |
| `getsentry/abacus` | [`package.json`](https://github.com/getsentry/abacus/blob/3e2f98567e1a59ecb92e1b925c5d70eeda9cd2c0/package.json#L53) | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/abacus` | [`pnpm-lock.yaml`](https://github.com/getsentry/abacus/blob/3e2f98567e1a59ecb92e1b925c5d70eeda9cd2c0/pnpm-lock.yaml#L6001) | locked transitive version: `2.58.4` |
| `getsentry/cli` | [`docs/pnpm-lock.yaml`](https://github.com/getsentry/cli/blob/d9bcd70eaa467fb3ddf591bfbfb0686fd1e9c016/docs/pnpm-lock.yaml#L3519) | locked transitive version: `2.58.6` |
| `getsentry/courses-app-sentry-nextjs` | [`package-lock.json`](https://github.com/getsentry/courses-app-sentry-nextjs/blob/78e88e515f6527ce0837dea784c2ba73767ff3b7/package-lock.json#L2783) | locked transitive version: `2.42.2` |
| `getsentry/craft` | [`pnpm-lock.yaml`](https://github.com/getsentry/craft/blob/a4e09a4a6729260c5067688601105de2765eac9e/pnpm-lock.yaml#L4501) | locked transitive version: `2.39.1` |
| `getsentry/dev-hub` | [`pnpm-lock.yaml`](https://github.com/getsentry/dev-hub/blob/487c13db2eb104139b57f891c69ccc3df13fe08e/pnpm-lock.yaml#L3424) | locked transitive version: `2.33.1` |
| `getsentry/downtime-simulator` | [`pnpm-lock.yaml`](https://github.com/getsentry/downtime-simulator/blob/91dd5ff12893e633416ed13f91ac3364506bbec0/pnpm-lock.yaml#L5227) | locked transitive version: `2.51.1` |
| `getsentry/error-generator` | [`pnpm-lock.yaml`](https://github.com/getsentry/error-generator/blob/76cdcaa4f90466c2bc611cd15f2b9cb449b4edf5/pnpm-lock.yaml#L3908) | locked transitive version: `2.42.2` |
| `getsentry/frontend-tutorial` | [`package-lock.json`](https://github.com/getsentry/frontend-tutorial/blob/ce68e195fa626706a710d200dda3a14e7fdb8614/package-lock.json#L1982) | locked transitive range: `^2.22.3` |
| `getsentry/gib-potato` | [`package-lock.json`](https://github.com/getsentry/gib-potato/blob/eaeed62564be96a033f2cabb5d43ad7751ac11ec/package-lock.json#L1456) | locked transitive range: `^2.57.0` |
| `getsentry/hackweek-wtfy` | [`pnpm-lock.yaml`](https://github.com/getsentry/hackweek-wtfy/blob/dee2c41b69a4988d17ea6414011abdda1d45700c/pnpm-lock.yaml#L3927) | locked transitive version: `2.52.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`package-lock.json`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/f8b07771aa05a9638de242b5c3a4075307a781ef/package-lock.json#L2232) | locked transitive range: `^2.51.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`pnpm-lock.yaml`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/f8b07771aa05a9638de242b5c3a4075307a781ef/pnpm-lock.yaml#L3572) | locked transitive version: `2.57.0` |
| `getsentry/nextjs-conf-scheduler` | [`pnpm-workspace.yaml`](https://github.com/getsentry/nextjs-conf-scheduler/blob/cdae1135e0823015c29f6a442a166511e4787dee/pnpm-workspace.yaml#L6) | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/nextjs-conf-scheduler` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-conf-scheduler/blob/cdae1135e0823015c29f6a442a166511e4787dee/pnpm-lock.yaml#L6416) | locked transitive version: `2.58.5` |
| `getsentry/nextjs-spotlight-test` | [`package-lock.json`](https://github.com/getsentry/nextjs-spotlight-test/blob/43ea0d57a7a0b1817ebeb01943e82f021ebdd9fb/package-lock.json#L2077) | locked transitive version: `2.39.1` |
| `getsentry/nextjs-spotlight-test` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-spotlight-test/blob/43ea0d57a7a0b1817ebeb01943e82f021ebdd9fb/pnpm-lock.yaml#L3443) | locked transitive version: `2.39.1` |
| `getsentry/sentry` | [`package.json`](https://github.com/getsentry/sentry/blob/787eb059f6796b09ffd80630c08d37a98c7b4087/package.json#L327) | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/sentry-build-academy-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-academy-guide/blob/ecf86b80e554739411279e16b7c8dc7fa81bc26e/pnpm-lock.yaml#L3728) | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`package-lock.json`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/9f95c66995bc20cebd5a1b0a90f1f422c5a3a1ff/package-lock.json#L2618) | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/9f95c66995bc20cebd5a1b0a90f1f422c5a3a1ff/pnpm-lock.yaml#L4856) | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-frontend-performance-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-frontend-performance-workshop-guide/blob/01309ebad3f3c7a27be9da89657a973818343d46/pnpm-lock.yaml#L4996) | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-otlp-workshop` | [`frontend/package-lock.json`](https://github.com/getsentry/sentry-build-otlp-workshop/blob/0d834a695e55e012c41e296f13a8e29cc533c2dd/frontend/package-lock.json#L1416) | locked transitive range: `^2.57.0` |
| `getsentry/sentry-build-otlp-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-otlp-workshop-guide/blob/61fef6c88f6f89322b2a9a54f36c09da8c706a71/pnpm-lock.yaml#L3728) | locked transitive version: `2.39.1` |
| `getsentry/sentry-crons-examples` | [`typescript/next/crons-nextjs-example/package-lock.json`](https://github.com/getsentry/sentry-crons-examples/blob/b2657f2a3f330d5db1c50fae1cdc060524d16d8b/typescript/next/crons-nextjs-example/package-lock.json#L2502) | locked transitive range: `^2.49.0` |
| `getsentry/sentry-docs` | [`package.json`](https://github.com/getsentry/sentry-docs/blob/2b7aa069a989fc80432a9f8adb48f067e9dd0cfd/package.json#L186) | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/vanguard` | [`pnpm-lock.yaml`](https://github.com/getsentry/vanguard/blob/30ddb7e9640e1083b183fec461835ff2ba9c8d93/pnpm-lock.yaml#L5650) | locked transitive version: `2.58.5` |
| `getsentry/sentry-changelog` | [`pnpm-workspace.yaml`](https://github.com/getsentry/sentry-changelog/blob/dd8bc4da7e3c609ae2f7e6ec7391cba30f000ee8/pnpm-workspace.yaml#L5) | pnpm build approval / workspace config includes `@sentry/cli` |
| `getsentry/sentry-toolbar` | [`pnpm-workspace.yaml`](https://github.com/getsentry/sentry-toolbar/blob/fdd7980ee54f198586758fa83e0d2875e95a008d/pnpm-workspace.yaml#L2) | pnpm workspace entry includes `@sentry/cli` |

## Fixtures, samples, and the source package

| Repo | Location | Reference |
| --- | --- | --- |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/Dockerfile`](https://github.com/getsentry/sentinel/blob/1b44ca947aa685b90b882013bfaea691a5358f8e/tests/fixtures/sample-code/Dockerfile#L29) | sample code installs `@sentry/cli` globally |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/Makefile`](https://github.com/getsentry/sentinel/blob/1b44ca947aa685b90b882013bfaea691a5358f8e/tests/fixtures/sample-code/Makefile#L166) | sample code installs `@sentry/cli` globally |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/example.sh`](https://github.com/getsentry/sentinel/blob/1b44ca947aa685b90b882013bfaea691a5358f8e/tests/fixtures/sample-code/example.sh#L82) | sample install instruction for `@sentry/cli` |
| `getsentry/sentry-cli` | [`package.json`](https://github.com/getsentry/sentry-cli/blob/49f87258d9a38b1ac3ba9e91051387905239e1c7/package.json#L2) | source npm package `@sentry/cli`; platform packages are declared in the same manifest |
