# Sentry CLI consumers found in `getsentry`

Scope: public `getsentry` repositories accessible through GitHub code search. I omitted documentation-only references, `getsentry/sentry-cli` itself, and `getsentry/sentry-release-registry` historical release metadata.

## Direct package-manager dependencies

| Repo | Location | Version/pin found | Notes |
| --- | --- | --- | --- |
| `getsentry/sentry-javascript` | [`packages/remix/package.json`](https://github.com/getsentry/sentry-javascript/blob/0a8adc4f67dcb4eedbc4a5454dad49a0d9d5305d/packages/remix/package.json#L71) | `^2.58.6` | Runtime dependency |
| `getsentry/sentry-javascript` | [`packages/react-router/package.json`](https://github.com/getsentry/sentry-javascript/blob/0a8adc4f67dcb4eedbc4a5454dad49a0d9d5305d/packages/react-router/package.json#L53) | `^2.58.6` | Runtime dependency |
| `getsentry/sentry-javascript-bundler-plugins` | [`packages/bundler-plugin-core/package.json`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/ac484d82fd9f3259d58acc421b4971dd4e5b46ce/packages/bundler-plugin-core/package.json#L57) | `^2.58.5` | Runtime dependency |
| `getsentry/action-release` | [`package.json`](https://github.com/getsentry/action-release/blob/6057f7e45acda771603cde6e19e0fe30310566e8/package.json#L32) | `^2.58.4` | Runtime dependency |
| `getsentry/eng-pipes` | [`package.json`](https://github.com/getsentry/eng-pipes/blob/32ffc46499bbb7c9d2db2235fc5b6ed34455d33e/package.json#L74) | `^2.20.1` | Dev dependency |
| `getsentry/plausible-mcp` | [`package.json`](https://github.com/getsentry/plausible-mcp/blob/2e0b3749a62cbdc673a178e809c2d828236b68f0/package.json#L36) | `^2` | Dev dependency |
| `getsentry/sentry-capacitor` | [`example/ionic-vue3/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-vue3/package.json#L31) | `^2.58.4` | Example app dependency |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v7/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-angular-v7/package.json#L32) | `^2.21.2` | Example app dev dependency |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v8/package.json`](https://github.com/getsentry/sentry-capacitor/blob/91e84dbf897fe93ac7047dc11745bbe3d7c73a88/example/ionic-angular-v8/package.json#L32) | `^2.21.2` | Example app dev dependency |
| `getsentry/sentry-react-native` | [`package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/package.json#L33) | `3.4.3` | Root package dependency |
| `getsentry/sentry-react-native` | [`packages/core/package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/packages/core/package.json#L78) | `3.4.3` | Core package dependency |
| `getsentry/sentry-react-native` | [`packages/expo-upload-sourcemaps/package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/packages/expo-upload-sourcemaps/package.json#L30) | `3.4.3` | Expo upload package dependency |
| `getsentry/wif` | [`package.json`](https://github.com/getsentry/wif/blob/c176c82f0bfbb1f9a9d8168870b5cbd645162d13/package.json#L46) | `^3.2.0` | Dev dependency |

## Non-NPM pinned CLI distribution consumers

| Repo | Location | Version/pin found | Notes |
| --- | --- | --- | --- |
| `getsentry/sentry-dart-plugin` | [`lib/src/cli/_sources.dart`](https://github.com/getsentry/sentry-dart-plugin/blob/cafaabeb46b96991dd2899c5f8fb127ec0083815/lib/src/cli/_sources.dart#L7) | `2.52.0` | Generated binary source list |
| `getsentry/sentry-android-gradle-plugin` | [`plugin-build/sentry-cli.properties`](https://github.com/getsentry/sentry-android-gradle-plugin/blob/aa8a9d0ec255a943042593485ae38cea1d23a6cf/plugin-build/sentry-cli.properties#L1) | `3.4.3` | Gradle plugin CLI downloader |
| `getsentry/sentry-maven-plugin` | [`sentry-cli.properties`](https://github.com/getsentry/sentry-maven-plugin/blob/2ac708c27292f4bacf55ca320080e4e50f056448/sentry-cli.properties#L1) | `3.3.0` | Maven plugin CLI downloader |
| `getsentry/sentry-fastlane-plugin` | [`script/sentry-cli.properties`](https://github.com/getsentry/sentry-fastlane-plugin/blob/591cb0e39fa89ecac44cadc20f775cc3b7623275/script/sentry-cli.properties#L1) | `3.4.3` | Fastlane plugin CLI downloader |
| `getsentry/sentry-unity` | [`modules/sentry-cli.properties`](https://github.com/getsentry/sentry-unity/blob/87a73b9ab522909dbafc183ec8ddc011a73d4173/modules/sentry-cli.properties#L1) | `3.4.3` | Unity CLI downloader |
| `getsentry/sentry-unreal` | [`plugin-dev/sentry-cli.properties`](https://github.com/getsentry/sentry-unreal/blob/8eeed3826c577f73fe2bf6689e969f2b97b3bbc9/plugin-dev/sentry-cli.properties#L1) | `3.4.3` | Unreal plugin CLI downloader |
| `getsentry/unreal-tower` | [`Plugins/Sentry/sentry-cli.properties`](https://github.com/getsentry/unreal-tower/blob/66f1be33767a25a5a5ed938340de80089cba564f/Plugins/Sentry/sentry-cli.properties#L1) | `3.4.2` | Unreal plugin sample/project copy |
| `getsentry/sentry-dotnet` | [`Directory.Build.props`](https://github.com/getsentry/sentry-dotnet/blob/691f3cfe93390c41147f83029f89271c420206c3/Directory.Build.props#L103) | `3.4.3` | NuGet package CLI binary download |
| `getsentry/homebrew-tools` | [`Formula/sentry-cli.rb`](https://github.com/getsentry/homebrew-tools/blob/8167f7f8ad1b08dfc9c2c14dfc077e085e1df29d/Formula/sentry-cli.rb#L4) | `3.4.3` | Homebrew formula version |
| `getsentry/self-hosted` | [`action.yaml`](https://github.com/getsentry/self-hosted/blob/4e2c7ae8896e770f8cdfae0fca255ae7d4e782a1/action.yaml#L220) | `3.4.1` | Installer script sets `SENTRY_CLI_VERSION` |

## Lockfile-only package-manager consumers

These entries are lockfile pins or package-manager metadata. They may be transitive through another Sentry package rather than direct `package.json` dependencies.

| Repo | Location | Version/pin found |
| --- | --- | --- |
| `getsentry/abacus` | [`pnpm-lock.yaml`](https://github.com/getsentry/abacus/blob/3e2f98567e1a59ecb92e1b925c5d70eeda9cd2c0/pnpm-lock.yaml#L1554) | `2.58.4` |
| `getsentry/cli` | [`docs/pnpm-lock.yaml`](https://github.com/getsentry/cli/blob/d9bcd70eaa467fb3ddf591bfbfb0686fd1e9c016/docs/pnpm-lock.yaml#L1061) | `2.58.6` |
| `getsentry/courses-app-sentry-nextjs` | [`package-lock.json`](https://github.com/getsentry/courses-app-sentry-nextjs/blob/78e88e515f6527ce0837dea784c2ba73767ff3b7/package-lock.json#L2783) | `2.42.2` |
| `getsentry/craft` | [`pnpm-lock.yaml`](https://github.com/getsentry/craft/blob/a4e09a4a6729260c5067688601105de2765eac9e/pnpm-lock.yaml#L1421) | `2.39.1` |
| `getsentry/dev-hub` | [`pnpm-lock.yaml`](https://github.com/getsentry/dev-hub/blob/487c13db2eb104139b57f891c69ccc3df13fe08e/pnpm-lock.yaml#L3424) | `2.33.1` |
| `getsentry/downtime-simulator` | [`pnpm-lock.yaml`](https://github.com/getsentry/downtime-simulator/blob/91dd5ff12893e633416ed13f91ac3364506bbec0/pnpm-lock.yaml#L1390) | `2.51.1` |
| `getsentry/error-generator` | [`pnpm-lock.yaml`](https://github.com/getsentry/error-generator/blob/76cdcaa4f90466c2bc611cd15f2b9cb449b4edf5/pnpm-lock.yaml#L870) | `2.42.2` |
| `getsentry/frontend-tutorial` | [`package-lock.json`](https://github.com/getsentry/frontend-tutorial/blob/ce68e195fa626706a710d200dda3a14e7fdb8614/package-lock.json#L1982) | `^2.22.3` |
| `getsentry/gib-potato` | [`package-lock.json`](https://github.com/getsentry/gib-potato/blob/eaeed62564be96a033f2cabb5d43ad7751ac11ec/package-lock.json#L1456) | `^2.57.0` |
| `getsentry/hackweek-wtfy` | [`pnpm-lock.yaml`](https://github.com/getsentry/hackweek-wtfy/blob/dee2c41b69a4988d17ea6414011abdda1d45700c/pnpm-lock.yaml#L1105) | `2.52.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`package-lock.json`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/f8b07771aa05a9638de242b5c3a4075307a781ef/package-lock.json#L2232) | `^2.51.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`pnpm-lock.yaml`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/f8b07771aa05a9638de242b5c3a4075307a781ef/pnpm-lock.yaml#L831) | `2.57.0` |
| `getsentry/nextjs-conf-scheduler` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-conf-scheduler/blob/cdae1135e0823015c29f6a442a166511e4787dee/pnpm-lock.yaml#L1905) | `2.58.5` |
| `getsentry/nextjs-spotlight-test` | [`package-lock.json`](https://github.com/getsentry/nextjs-spotlight-test/blob/43ea0d57a7a0b1817ebeb01943e82f021ebdd9fb/package-lock.json#L2077) | `2.39.1` |
| `getsentry/nextjs-spotlight-test` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-spotlight-test/blob/43ea0d57a7a0b1817ebeb01943e82f021ebdd9fb/pnpm-lock.yaml#L778) | `2.39.1` |
| `getsentry/plausible-mcp` | [`bun.lock`](https://github.com/getsentry/plausible-mcp/blob/2e0b3749a62cbdc673a178e809c2d828236b68f0/bun.lock#L15) | `^2` request, resolved later in the lockfile |
| `getsentry/sentry-build-academy-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-academy-guide/blob/ecf86b80e554739411279e16b7c8dc7fa81bc26e/pnpm-lock.yaml#L876) | `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`package-lock.json`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/9f95c66995bc20cebd5a1b0a90f1f422c5a3a1ff/package-lock.json#L2618) | `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/9f95c66995bc20cebd5a1b0a90f1f422c5a3a1ff/pnpm-lock.yaml#L983) | `2.39.1` |
| `getsentry/sentry-build-frontend-performance-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-frontend-performance-workshop-guide/blob/01309ebad3f3c7a27be9da89657a973818343d46/pnpm-lock.yaml#L1023) | `2.39.1` |
| `getsentry/sentry-build-otlp-workshop` | [`frontend/package-lock.json`](https://github.com/getsentry/sentry-build-otlp-workshop/blob/0d834a695e55e012c41e296f13a8e29cc533c2dd/frontend/package-lock.json#L1416) | `^2.57.0` |
| `getsentry/sentry-build-otlp-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-otlp-workshop-guide/blob/61fef6c88f6f89322b2a9a54f36c09da8c706a71/pnpm-lock.yaml#L876) | `2.39.1` |
| `getsentry/sentry-crons-examples` | [`typescript/next/crons-nextjs-example/package-lock.json`](https://github.com/getsentry/sentry-crons-examples/blob/b2657f2a3f330d5db1c50fae1cdc060524d16d8b/typescript/next/crons-nextjs-example/package-lock.json#L2502) | `^2.49.0` |
| `getsentry/vanguard` | [`pnpm-lock.yaml`](https://github.com/getsentry/vanguard/blob/30ddb7e9640e1083b183fec461835ff2ba9c8d93/pnpm-lock.yaml#L2053) | `2.58.5` |
| `getsentry/wif` | [`pnpm-lock.yaml`](https://github.com/getsentry/wif/blob/c176c82f0bfbb1f9a9d8168870b5cbd645162d13/pnpm-lock.yaml#L957) | `3.2.0` |

## Dynamic CLI invocations without repo-pinned CLI versions

These repos invoke `sentry-cli` or provide download helpers, but I did not find a pinned CLI version in the invocation itself.

| Repo | Location | Behavior |
| --- | --- | --- |
| `getsentry/app-runner` | [`sentry-api-client/Public/Get-SentryCLI.ps1`](https://github.com/getsentry/app-runner/blob/fd2f57b49982597557fefa95c5915c623e1becff/sentry-api-client/Public/Get-SentryCLI.ps1#L39) | Defaults to downloading `latest`; callers can pass a version |
| `getsentry/app-runner` | [`sentry-api-client/Public/Invoke-SentryCLI.ps1`](https://github.com/getsentry/app-runner/blob/fd2f57b49982597557fefa95c5915c623e1becff/sentry-api-client/Public/Invoke-SentryCLI.ps1#L62) | Defaults to `system`; callers can pass `latest` or a semantic version |
| `getsentry/relay` | [`Makefile`](https://github.com/getsentry/relay/blob/1d9b84fb47666af09e866dd64d43617b6506ed10/Makefile#L36) | Invokes `sentry-cli` from `PATH` |
| `getsentry/uptime-checker` | [`scripts/upload-debug-symbols`](https://github.com/getsentry/uptime-checker/blob/5495026f7a2f76ed494c1cfb3a4b050104a1ae76/scripts/upload-debug-symbols#L33) | Invokes `sentry-cli` from `PATH` |
| `getsentry/symbolicator` | [`scripts/create-sentry-release`](https://github.com/getsentry/symbolicator/blob/55dd169103a2d1696f371c4ddf3de89512df8268/scripts/create-sentry-release#L22) | Invokes `sentry-cli` from `PATH` |
| `getsentry/sentry-mobile-release-health-app` | [`ios/fastlane/Fastfile`](https://github.com/getsentry/sentry-mobile-release-health-app/blob/52c8df39094c7179a1dc2d37fe88d6f980694ca7/ios/fastlane/Fastfile#L119) | Invokes `sentry-cli` from `PATH` |

## Package-manager build-script allowlists

These entries allow `@sentry/cli` install scripts in pnpm metadata, but they are not version pins by themselves.

| Repo | Location |
| --- | --- |
| `getsentry/abacus` | [`package.json`](https://github.com/getsentry/abacus/blob/3e2f98567e1a59ecb92e1b925c5d70eeda9cd2c0/package.json#L53) |
| `getsentry/sentry` | [`package.json`](https://github.com/getsentry/sentry/blob/a8862f21989ee0587aeef715015e0be50ce61daf/package.json#L327) |
| `getsentry/sentry-docs` | [`package.json`](https://github.com/getsentry/sentry-docs/blob/2b7aa069a989fc80432a9f8adb48f067e9dd0cfd/package.json#L186) |
| `getsentry/wif` | [`package.json`](https://github.com/getsentry/wif/blob/c176c82f0bfbb1f9a9d8168870b5cbd645162d13/package.json#L40) |
