# Sentry CLI consumer snapshot in `getsentry`

This document contains all usages of `getsentry/sentry-cli` we were able to find in **public GitHub repos under the `getsentry` org** as of **2026-05-27**.

> [!NOTE]
> 🤖 AI-generated artifact. This snapshot is only minimally validated and should be treated as a best-effort attempt. The manually checked entries look accurate, but it may contain inaccuracies or be incomplete.

Two agents independently generated the two source inventories using GitHub code searches. A third agent merged them into this README and also searched for additional `getsentry` locations that may have been missed.

## Directory contents

- [README.md](README.md) — final merged snapshot summary for the directory; includes the source inventories and the merged results.
- [build-snapshot.py](build-snapshot.py) — script that generated the snapshot bundle and verified checklist coverage.
- [inventory-01-first.md](inventory-01-first.md) — first independently generated source inventory from GitHub search.
- [inventory-02-second.md](inventory-02-second.md) — second independently generated source inventory from GitHub search.
- [repo-checklist.md](repo-checklist.md) — intermediate repo checklist used to verify coverage during consolidation.

## Direct, installer, or pinned binary consumers

| Repo | Location | Version/pin | Notes |
| --- | --- | --- | --- |
| `getsentry/action-release` | [`action.yml`](https://github.com/getsentry/action-release/blob/f71adb49d4b2aeeda98052d3de234bbb0f3e03ab/action.yml#L205) | `^2.4` | runtime install: `npm install --no-package-lock @sentry/cli@^2.4` |
| `getsentry/action-release` | [`package.json`](https://github.com/getsentry/action-release/blob/f71adb49d4b2aeeda98052d3de234bbb0f3e03ab/package.json#L31) | `^2.58.6` | npm dependency: `"@sentry/cli": "^2.58.4"` |
| `getsentry/eng-pipes` | [`package.json`](https://github.com/getsentry/eng-pipes/blob/32ffc46499bbb7c9d2db2235fc5b6ed34455d33e/package.json#L74) | `^2.20.1` | dev dependency: `"@sentry/cli": "^2.20.1"` |
| `getsentry/homebrew-tools` | [`Formula/sentry-cli.rb`](https://github.com/getsentry/homebrew-tools/blob/8167f7f8ad1b08dfc9c2c14dfc077e085e1df29d/Formula/sentry-cli.rb#L3) | `3.4.3` | `3.4.3`  \| Homebrew formula version |
| `getsentry/plausible-mcp` | [`bun.lock`](https://github.com/getsentry/plausible-mcp/blob/6c59e3291d4a2f5a4e4cfabefb831b9163fa0c05/bun.lock#L15) | `^2` | `^2` request, resolved later in the lockfile |
| `getsentry/plausible-mcp` | [`package.json`](https://github.com/getsentry/plausible-mcp/blob/2e0b3749a62cbdc673a178e809c2d828236b68f0/package.json#L36) | `^2` | dev dependency: `"@sentry/cli": "^2"` |
| `getsentry/self-hosted` | [`action.yaml`](https://github.com/getsentry/self-hosted/blob/afe558b0e27a51c05675c7c51254230d37856820/action.yaml#L220) | `3.4.1` | `3.4.1`  \| Installer script sets `SENTRY_CLI_VERSION` |
| `getsentry/sentry-android-gradle-plugin` | [`plugin-build/sentry-cli.properties`](https://github.com/getsentry/sentry-android-gradle-plugin/blob/3a7c4decd0fdc22469fe88b7f2eb2f3d6195c5c3/plugin-build/sentry-cli.properties#L2) | `3.4.3` | `3.4.3`  \| Gradle plugin CLI downloader |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v7/package.json`](https://github.com/getsentry/sentry-capacitor/blob/743adb0b2ab58d43d494d9319d1b0c2ea796a183/example/ionic-angular-v7/package.json#L32) | `^2.21.2` | dev dependency: `"@sentry/cli": "^2.21.2"` |
| `getsentry/sentry-capacitor` | [`example/ionic-angular-v8/package.json`](https://github.com/getsentry/sentry-capacitor/blob/743adb0b2ab58d43d494d9319d1b0c2ea796a183/example/ionic-angular-v8/package.json#L32) | `^2.21.2` | dev dependency: `"@sentry/cli": "^2.21.2"` |
| `getsentry/sentry-capacitor` | [`example/ionic-vue3/package.json`](https://github.com/getsentry/sentry-capacitor/blob/743adb0b2ab58d43d494d9319d1b0c2ea796a183/example/ionic-vue3/package.json#L31) | `^2.58.4` | dependency: `"@sentry/cli": "^2.58.4"` |
| `getsentry/sentry-dart-plugin` | [`lib/src/cli/_sources.dart`](https://github.com/getsentry/sentry-dart-plugin/blob/fb06747d163e525062ded20d0c83d645c4c44dea/lib/src/cli/_sources.dart#L11) | `2.52.0` | hardcoded Sentry CLI binary source version: `2.52.0` |
| `getsentry/sentry-dotnet` | [`Directory.Build.props`](https://github.com/getsentry/sentry-dotnet/blob/369de600bace8b771e368dca21b8880e552aa1a5/Directory.Build.props#L104) | `3.4.3` | `3.4.3`  \| NuGet package CLI binary download |
| `getsentry/sentry-fastlane-plugin` | [`script/sentry-cli.properties`](https://github.com/getsentry/sentry-fastlane-plugin/blob/591cb0e39fa89ecac44cadc20f775cc3b7623275/script/sentry-cli.properties#L2) | `3.4.3` | `3.4.3`  \| Fastlane plugin CLI downloader |
| `getsentry/sentry-javascript` | [`packages/react-router/package.json`](https://github.com/getsentry/sentry-javascript/blob/2ae69d8b8e5eb57c69b0656288fc64b23e99b5d7/packages/react-router/package.json#L53) | `^2.58.6` | dependency: `"@sentry/cli": "^2.58.6"` |
| `getsentry/sentry-javascript` | [`packages/remix/package.json`](https://github.com/getsentry/sentry-javascript/blob/2ae69d8b8e5eb57c69b0656288fc64b23e99b5d7/packages/remix/package.json#L71) | `^2.58.6` | dependency: `"@sentry/cli": "^2.58.6"` |
| `getsentry/sentry-javascript-bundler-plugins` | [`packages/bundler-plugin-core/package.json`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/1099a5fb048595e45a14496f67b55bbb8a950fb9/packages/bundler-plugin-core/package.json#L57) | `^2.58.6` | dependency: `"@sentry/cli": "^2.58.5"` |
| `getsentry/sentry-maven-plugin` | [`sentry-cli.properties`](https://github.com/getsentry/sentry-maven-plugin/blob/120b17d2c1760639d122f9df1193f3bff9f7a266/sentry-cli.properties#L2) | `3.3.0` | `3.3.0`  \| Maven plugin CLI downloader |
| `getsentry/sentry-react-native` | [`.github/workflows/e2e-v2.yml`](https://github.com/getsentry/sentry-react-native/blob/dc9765dc655878859a489a76712a3984972e62c0/.github/workflows/e2e-v2.yml#L123) | `unpinned / supplied by environment` | CI install: `npm i -g react-native-cli @sentry/cli` |
| `getsentry/sentry-react-native` | [`package.json`](https://github.com/getsentry/sentry-react-native/blob/89241951bce9e3f03fc362a83bee32f8ea3db724/package.json#L33) | `3.4.3` | root dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-react-native` | [`packages/core/package.json`](https://github.com/getsentry/sentry-react-native/blob/eb93136bf3342a029dec25eba5cc871a8007a458/packages/core/package.json#L78) | `3.4.3` | package dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-react-native` | [`packages/expo-upload-sourcemaps/package.json`](https://github.com/getsentry/sentry-react-native/blob/153d0a2417f5c88ba392be5a034a148e4beca600/packages/expo-upload-sourcemaps/package.json#L30) | `3.4.3` | package dependency: `"@sentry/cli": "3.4.3"` |
| `getsentry/sentry-unity` | [`modules/sentry-cli.properties`](https://github.com/getsentry/sentry-unity/blob/3d7e9c6d94abe05c3a67283f239e9ce3015f9f55/modules/sentry-cli.properties#L2) | `3.4.3` | `3.4.3`  \| Unity CLI downloader |
| `getsentry/sentry-unreal` | [`plugin-dev/sentry-cli.properties`](https://github.com/getsentry/sentry-unreal/blob/8eeed3826c577f73fe2bf6689e969f2b97b3bbc9/plugin-dev/sentry-cli.properties#L2) | `3.4.3` | `3.4.3`  \| Unreal plugin CLI downloader |
| `getsentry/sentry-wizard` | [`lib/Helper/SentryCli.ts`](https://github.com/getsentry/sentry-wizard/blob/6f81c62fb6d63c93447c1850530aa7386c4ac116/lib/Helper/SentryCli.ts#L38) | `not pinned here` | resolves @sentry/cli/bin/sentry-cli |
| `getsentry/sentry-wizard` | [`src/sourcemaps/tools/sentry-cli.ts`](https://github.com/getsentry/sentry-wizard/blob/master/src/sourcemaps/tools/sentry-cli.ts#L38) | `^2` | installs/uses @sentry/cli for sourcemaps |
| `getsentry/unreal-tower` | [`Plugins/Sentry/sentry-cli.properties`](https://github.com/getsentry/unreal-tower/blob/66f1be33767a25a5a5ed938340de80089cba564f/Plugins/Sentry/sentry-cli.properties#L2) | `3.4.2` | `3.4.2`  \| Unreal plugin sample/project copy |
| `getsentry/wif` | [`package.json`](https://github.com/getsentry/wif/blob/49dc995d43365a9e87c18c886f59acdd2706164f/package.json#L40) | `^3.2.0` | dev dependency: `"@sentry/cli": "^3.2.0"` |
| `getsentry/wif` | [`pnpm-lock.yaml`](https://github.com/getsentry/wif/blob/062283018991065e1ba45579da3ec5b2f100dfa6/pnpm-lock.yaml#L60) | `3.2.0` | `3.2.0` |

## Lockfile-only or package-manager metadata consumers

| Repo | Location | Version/pin | Notes |
| --- | --- | --- | --- |
| `getsentry/abacus` | [`package.json`](https://github.com/getsentry/abacus/blob/7470f28ff274c22658497684f17f5e1c67dac6e2/package.json#L53) | `not pinned in this metadata` | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/abacus` | [`pnpm-lock.yaml`](https://github.com/getsentry/abacus/blob/7470f28ff274c22658497684f17f5e1c67dac6e2/pnpm-lock.yaml#L1507) | `2.58.4` | locked transitive version: `2.58.4` |
| `getsentry/cli` | [`docs/pnpm-lock.yaml`](https://github.com/getsentry/cli/blob/c06454796ab615ab6b7e732287d9debad5286c19/docs/pnpm-lock.yaml#L1014) | `2.58.6` | locked transitive version: `2.58.6` |
| `getsentry/courses-app-sentry-nextjs` | [`package-lock.json`](https://github.com/getsentry/courses-app-sentry-nextjs/blob/a7febf92421fa1890145be704b7115885b39edbe/package-lock.json#L2783) | `2.42.2` | locked transitive version: `2.42.2` |
| `getsentry/craft` | [`pnpm-lock.yaml`](https://github.com/getsentry/craft/blob/2662e81254403b708b6e5c33867023970065637d/pnpm-lock.yaml#L1380) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/dev-hub` | [`pnpm-lock.yaml`](https://github.com/getsentry/dev-hub/blob/465e06f8b68412c0b7480418b38a4299c7ac504f/pnpm-lock.yaml#L3424) | `2.33.1` | locked transitive version: `2.33.1` |
| `getsentry/downtime-simulator` | [`pnpm-lock.yaml`](https://github.com/getsentry/downtime-simulator/blob/ab161aafb77539c9801ee4d058bd807bdfac683b/pnpm-lock.yaml#L1343) | `2.51.1` | locked transitive version: `2.51.1` |
| `getsentry/error-generator` | [`pnpm-lock.yaml`](https://github.com/getsentry/error-generator/blob/76cdcaa4f90466c2bc611cd15f2b9cb449b4edf5/pnpm-lock.yaml#L829) | `2.42.2` | locked transitive version: `2.42.2` |
| `getsentry/frontend-tutorial` | [`package-lock.json`](https://github.com/getsentry/frontend-tutorial/blob/ce68e195fa626706a710d200dda3a14e7fdb8614/package-lock.json#L1982) | `^2.22.3` | locked transitive range: `^2.22.3` |
| `getsentry/gib-potato` | [`package-lock.json`](https://github.com/getsentry/gib-potato/blob/eaeed62564be96a033f2cabb5d43ad7751ac11ec/package-lock.json#L1456) | `^2.57.0` | locked transitive range: `^2.57.0` |
| `getsentry/hackweek-wtfy` | [`pnpm-lock.yaml`](https://github.com/getsentry/hackweek-wtfy/blob/ea9d2d19adacb1d55af846a33c959f27979e7f7c/pnpm-lock.yaml#L1058) | `2.52.0` | locked transitive version: `2.52.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`package-lock.json`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/d9d17324edac72078b75a52d9a24b8d50e7ebd18/package-lock.json#L2232) | `^2.51.0` | locked transitive range: `^2.51.0` |
| `getsentry/llm-manual-agent-monitoring-example` | [`pnpm-lock.yaml`](https://github.com/getsentry/llm-manual-agent-monitoring-example/blob/d9d17324edac72078b75a52d9a24b8d50e7ebd18/pnpm-lock.yaml#L784) | `2.57.0` | locked transitive version: `2.57.0` |
| `getsentry/nextjs-conf-scheduler` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-conf-scheduler/blob/08309fb1442b5cd4471c787a67dd0d8654b40b49/pnpm-lock.yaml#L1858) | `2.58.5` | locked transitive version: `2.58.5` |
| `getsentry/nextjs-conf-scheduler` | [`pnpm-workspace.yaml`](https://github.com/getsentry/nextjs-conf-scheduler/blob/d2d9b817a8a29211138aa77605aab61930070655/pnpm-workspace.yaml#L6) | `not pinned in this metadata` | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/nextjs-spotlight-test` | [`package-lock.json`](https://github.com/getsentry/nextjs-spotlight-test/blob/a0cf15d5311cc3d91ca9ea702b564bc38a3df1d2/package-lock.json#L2077) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/nextjs-spotlight-test` | [`pnpm-lock.yaml`](https://github.com/getsentry/nextjs-spotlight-test/blob/a0cf15d5311cc3d91ca9ea702b564bc38a3df1d2/pnpm-lock.yaml#L737) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry` | [`package.json`](https://github.com/getsentry/sentry/blob/b8f1a9033de464e100069f8d036b923dc8ad674d/package.json#L327) | `not pinned in this metadata` | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/sentry-build-academy-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-academy-guide/blob/39fc0c4a837606b01e56db6b968579f42a394649/pnpm-lock.yaml#L835) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`package-lock.json`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/c9579515e1ff5f06efedecefca08cb5f6603a084/package-lock.json#L2618) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-ai-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-ai-workshop-guide/blob/8efa6ef57ae482a8188fdc893e581196328cd8f3/pnpm-lock.yaml#L942) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-frontend-performance-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-frontend-performance-workshop-guide/blob/ea6a60013f81e76c7b398bb6e0ac184a3d95a1dc/pnpm-lock.yaml#L982) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry-build-otlp-workshop` | [`frontend/package-lock.json`](https://github.com/getsentry/sentry-build-otlp-workshop/blob/e01b3b707da4142f9c66bfcd4ce78d3bb000d484/frontend/package-lock.json#L1416) | `^2.57.0` | locked transitive range: `^2.57.0` |
| `getsentry/sentry-build-otlp-workshop-guide` | [`pnpm-lock.yaml`](https://github.com/getsentry/sentry-build-otlp-workshop-guide/blob/39fc0c4a837606b01e56db6b968579f42a394649/pnpm-lock.yaml#L835) | `2.39.1` | locked transitive version: `2.39.1` |
| `getsentry/sentry-changelog` | [`pnpm-workspace.yaml`](https://github.com/getsentry/sentry-changelog/blob/6c211c5eb103b9d3f09547b481d6e7a9fb75b6c8/pnpm-workspace.yaml#L5) | `not pinned in this metadata` | pnpm build approval / workspace config includes `@sentry/cli` |
| `getsentry/sentry-crons-examples` | [`typescript/next/crons-nextjs-example/package-lock.json`](https://github.com/getsentry/sentry-crons-examples/blob/54d0d1bf37469879da2bdce0e47d0c2a02775e20/typescript/next/crons-nextjs-example/package-lock.json#L2502) | `^2.49.0` | locked transitive range: `^2.49.0` |
| `getsentry/sentry-docs` | [`package.json`](https://github.com/getsentry/sentry-docs/blob/282ae846a517f68d53f4e9b460dc8a9524d9a4a8/package.json#L186) | `not pinned in this metadata` | pnpm `onlyBuiltDependencies`: `@sentry/cli` |
| `getsentry/sentry-toolbar` | [`pnpm-workspace.yaml`](https://github.com/getsentry/sentry-toolbar/blob/09593d8e412259b87ef1fcc545ede4f4f8a4f4f3/pnpm-workspace.yaml#L2) | `not pinned in this metadata` | pnpm workspace entry includes `@sentry/cli` |
| `getsentry/vanguard` | [`pnpm-lock.yaml`](https://github.com/getsentry/vanguard/blob/30ddb7e9640e1083b183fec461835ff2ba9c8d93/pnpm-lock.yaml#L2006) | `2.58.5` | locked transitive version: `2.58.5` |

## Dynamic CLI invocations without repo-pinned versions

| Repo | Location | Version/pin | Notes |
| --- | --- | --- | --- |
| `getsentry/app-runner` | [`sentry-api-client/Public/Get-SentryCLI.ps1`](https://github.com/getsentry/app-runner/blob/21dd249834a685f00ffed4a1fc3bd24a86bcc16f/sentry-api-client/Public/Get-SentryCLI.ps1#L68) | `unpinned / supplied by environment` | Defaults to downloading `latest`; callers can pass a version |
| `getsentry/app-runner` | [`sentry-api-client/Public/Invoke-SentryCLI.ps1`](https://github.com/getsentry/app-runner/blob/21dd249834a685f00ffed4a1fc3bd24a86bcc16f/sentry-api-client/Public/Invoke-SentryCLI.ps1#L27) | `unpinned / supplied by environment` | Defaults to `system`; callers can pass `latest` or a semantic version |
| `getsentry/relay` | [`Makefile`](https://github.com/getsentry/relay/blob/9ac6973a83f7791378ddbc0a935a003c4047615e/Makefile#L36) | `unpinned / supplied by environment` | Invokes `sentry-cli` from `PATH` |
| `getsentry/sentry-mobile-release-health-app` | [`ios/fastlane/Fastfile`](https://github.com/getsentry/sentry-mobile-release-health-app/blob/2fb61b008ecf1e3daed11ef8cdfc78a233127b07/ios/fastlane/Fastfile#L119) | `unpinned / supplied by environment` | Invokes `sentry-cli` from `PATH` |
| `getsentry/symbolicator` | [`scripts/create-sentry-release`](https://github.com/getsentry/symbolicator/blob/7d3d31ae3be6b2b70cf5d6cfc74ffda65b647462/scripts/create-sentry-release#L22) | `unpinned / supplied by environment` | Invokes `sentry-cli` from `PATH` |
| `getsentry/uptime-checker` | [`scripts/upload-debug-symbols`](https://github.com/getsentry/uptime-checker/blob/f4ac242d6c6a69e1a65e6288cf00c3c5cd1e4af0/scripts/upload-debug-symbols#L33) | `unpinned / supplied by environment` | Invokes `sentry-cli` from `PATH` |

## Fixtures, samples, metadata, and source package

| Repo | Location | Version/pin | Notes |
| --- | --- | --- | --- |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/Dockerfile`](https://github.com/getsentry/sentinel/blob/4d6ea0c933650cc830f6c0766b36c0fef724256a/tests/fixtures/sample-code/Dockerfile#L29) | `unpinned / supplied by environment` | sample code installs `@sentry/cli` globally |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/Makefile`](https://github.com/getsentry/sentinel/blob/4d6ea0c933650cc830f6c0766b36c0fef724256a/tests/fixtures/sample-code/Makefile#L166) | `unpinned / supplied by environment` | sample code installs `@sentry/cli` globally |
| `getsentry/sentinel` | [`tests/fixtures/sample-code/example.sh`](https://github.com/getsentry/sentinel/blob/4d6ea0c933650cc830f6c0766b36c0fef724256a/tests/fixtures/sample-code/example.sh#L82) | `unpinned / supplied by environment` | sample install instruction for `@sentry/cli` |
| `getsentry/sentry-cli` | [`package.json`](https://github.com/getsentry/sentry-cli/blob/ee5286953ea3b7e419e017c3c47f64e26a45ff19/package.json#L2) | `source package` | source npm package `@sentry/cli`; platform packages are declared in the same manifest |
