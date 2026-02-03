# Sentry CLI Distribution & Repackaging

This document consolidates all known ways Sentry CLI is distributed or repackaged, along with repositories that bundle or download it.

> [!NOTE]
> This document was updated on **2026-01-29** and is accurate as of that date. We do not intend to actively maintain this document; it should only be considered as a snapshot of the state of the Sentry CLI distribution ecosystem at that time.

## Release registry

- [**getsentry/sentry-release-registry**](https://github.com/getsentry/sentry-release-registry) — release registry listing Sentry CLI binaries and packages (tgz/whl/etc.) in [apps/sentry-cli/](https://github.com/getsentry/sentry-release-registry/tree/5bdf153b7935fe4f696762ea132a592feaeba849/apps/sentry-cli).
  - Registry endpoint: [Sentry CLI release registry](https://release-registry.services.sentry.io/apps/sentry-cli/latest)

## Distribution mechanisms in [the Sentry CLI repo](https://github.com/getsentry/sentry-cli)

- **NPM wrapper package** — `@sentry/cli` package definition and install script ([`package.json`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/package.json), [`scripts/install.js`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/scripts/install.js)).
  - Package: [`@sentry/cli`](https://www.npmjs.com/package/@sentry/cli)
- **Platform‑specific npm binary packages** — optional deps under `npm-binary-distributions/` (e.g. `@sentry/cli-linux-x64`, `@sentry/cli-win32-x64`).
  - Source: [`npm-binary-distributions/*/package.json`](https://github.com/getsentry/sentry-cli/tree/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/npm-binary-distributions)
  - Packages (examples): [`@sentry/cli-linux-x64`](https://www.npmjs.com/package/@sentry/cli-linux-x64), [`@sentry/cli-win32-x64`](https://www.npmjs.com/package/@sentry/cli-win32-x64)
- **Python package / wheels** — `sentry_cli` package built via `setuptools-rust` ([`setup.py`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/setup.py), [`setup.cfg`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/setup.cfg), [`pyproject.toml`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/pyproject.toml)).
- **GitHub Releases binaries** — prebuilt platform executables published as release assets.
  - Releases: [Sentry CLI GitHub releases](https://github.com/getsentry/sentry-cli/releases)
- **Docker image build** — Dockerfile produces an Alpine‑based image with `sentry-cli` in PATH ([`Dockerfile`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/Dockerfile), [`docker-entrypoint.sh`](https://github.com/getsentry/sentry-cli/blob/9c1cd63b07c7a79ba7345440a800f56e385e9fc0/docker-entrypoint.sh)).
  - Image: [Sentry CLI image on Docker Hub](https://hub.docker.com/r/getsentry/sentry-cli)

## Installer repos referenced by official docs (first‑party)

- [**getsentry/sentry**](https://github.com/getsentry/sentry) — serves the `get-cli` installer script endpoint ([`src/sentry/web/frontend/cli.py`](https://github.com/getsentry/sentry/blob/0be9af9839f465e25af8e0d8f8921c870e0f0d8b/src/sentry/web/frontend/cli.py)).
- [**getsentry/homebrew-tools**](https://github.com/getsentry/homebrew-tools) — Homebrew tap containing the `sentry-cli` formula referenced in docs ([`Formula/sentry-cli.rb`](https://github.com/getsentry/homebrew-tools/blob/cd5047cb8951788829f2e00ff368626c11fbfb48/Formula/sentry-cli.rb)).
- **Official installation docs** — list install methods (manual download, get‑cli script, npm, Homebrew tap, Scoop, Docker, update/uninstall).
  - Docs: [Sentry CLI installation](https://docs.sentry.io/cli/installation/)

## First‑party consumers that bundle or download CLI (SDKs/plugins/actions)

- **Android** — [getsentry/sentry-android-gradle-plugin](https://github.com/getsentry/sentry-android-gradle-plugin)
  - Version: [3.1.0](https://github.com/getsentry/sentry-android-gradle-plugin/blob/90c682f5575143ab379d5ac3bc5d24d161721773/plugin-build/sentry-cli.properties#L1)
  - Observed CLI usage: [`upload-proguard`](https://github.com/getsentry/sentry-android-gradle-plugin/blob/90c682f5575143ab379d5ac3bc5d24d161721773/plugin-build/src/main/kotlin/io/sentry/android/gradle/tasks/SentryUploadProguardMappingsTask.kt#L69), [`debug-files upload`](https://github.com/getsentry/sentry-android-gradle-plugin/blob/90c682f5575143ab379d5ac3bc5d24d161721773/plugin-build/src/main/kotlin/io/sentry/android/gradle/tasks/SentryUploadNativeSymbolsTask.kt#L37-L38), [`build upload`](https://github.com/getsentry/sentry-android-gradle-plugin/blob/90c682f5575143ab379d5ac3bc5d24d161721773/plugin-build/src/main/kotlin/io/sentry/android/gradle/tasks/SentryUploadAppArtifactTask.kt#L49-L50).
- **Java/Maven** — [getsentry/sentry-maven-plugin](https://github.com/getsentry/sentry-maven-plugin)
  - Package: [sentry-maven-plugin on Maven Central](https://maven-badges.herokuapp.com/maven-central/io.sentry/sentry-maven-plugin)
  - Version: [2.58.4](https://github.com/getsentry/sentry-maven-plugin/blob/d768f6a32d5a9dc459120e95ea2d93eca2988137/sentry-cli.properties#L1)
  - Observed CLI usage: [`debug-files bundle-jvm`](https://github.com/getsentry/sentry-maven-plugin/blob/d768f6a32d5a9dc459120e95ea2d93eca2988137/src/main/java/io/sentry/UploadSourceBundleMojo.java#L203-L204), [`debug-files upload --type=jvm`](https://github.com/getsentry/sentry-maven-plugin/blob/d768f6a32d5a9dc459120e95ea2d93eca2988137/src/main/java/io/sentry/UploadSourceBundleMojo.java#L249-L252).
- **Dart/Flutter** — [getsentry/sentry-dart-plugin](https://github.com/getsentry/sentry-dart-plugin)
  - Package: [`sentry_dart_plugin`](https://pub.dev/packages/sentry_dart_plugin)
  - Version: [2.52.0](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/src/cli/_sources.dart#L7)
  - Observed CLI usage: [`releases new`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/sentry_dart_plugin.dart#L168), [`releases finalize`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/sentry_dart_plugin.dart#L172-L173), [`releases set-commits`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/sentry_dart_plugin.dart#L178-L179), [`debug-files upload`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/sentry_dart_plugin.dart#L77-L78), [`sourcemaps upload`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/sentry_dart_plugin.dart#L265-L266), [`dart-symbol-map upload`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/src/symbol_maps/dart_symbol_map_uploader.dart#L66-L70), and [`debug-files check`](https://github.com/getsentry/sentry-dart-plugin/blob/4c7322157e23bdc7d078af9b255eed2771d8e8f5/lib/src/symbol_maps/dart_symbol_map_uploader.dart#L138-L143).
- **Fastlane/iOS** — [getsentry/sentry-fastlane-plugin](https://github.com/getsentry/sentry-fastlane-plugin)
  - Package: [`fastlane-plugin-sentry`](https://rubygems.org/gems/fastlane-plugin-sentry)
  - Version: [3.1.0](https://github.com/getsentry/sentry-fastlane-plugin/blob/c6e7a6134fe6f1505fb00c160d70e3fbd5dca467/script/sentry-cli.properties#L1)
  - Observed CLI usage: [`debug-files upload`](https://github.com/getsentry/sentry-fastlane-plugin/blob/c6e7a6134fe6f1505fb00c160d70e3fbd5dca467/lib/fastlane/plugin/sentry/actions/sentry_debug_files_upload.rb#L21-L22), [`sourcemaps upload`](https://github.com/getsentry/sentry-fastlane-plugin/blob/c6e7a6134fe6f1505fb00c160d70e3fbd5dca467/lib/fastlane/plugin/sentry/actions/sentry_upload_sourcemap.rb#L16-L17).
- **Unity** — [getsentry/sentry-unity](https://github.com/getsentry/sentry-unity)
  - Version: [3.1.0](https://github.com/getsentry/sentry-unity/blob/03fd79c889d0e3c7d2f532297b122c2018216d20/modules/sentry-cli.properties#L1)
  - Binary location: [`Editor/sentry-cli/`](https://github.com/getsentry/sentry-unity/blob/03fd79c889d0e3c7d2f532297b122c2018216d20/src/Sentry.Unity.Editor/SentryCli.cs#L75-L79)
  - Observed CLI usage: [`debug-files upload`](https://github.com/getsentry/sentry-unity/blob/03fd79c889d0e3c7d2f532297b122c2018216d20/src/Sentry.Unity.Editor/Android/DebugSymbolUpload.cs#L63), [`upload-proguard`](https://github.com/getsentry/sentry-unity/blob/03fd79c889d0e3c7d2f532297b122c2018216d20/src/Sentry.Unity.Editor/Android/DebugSymbolUpload.cs#L285).
- **Unreal** — [getsentry/sentry-unreal](https://github.com/getsentry/sentry-unreal)
  - Downloads: [Sentry Unreal releases](https://github.com/getsentry/sentry-unreal/releases), [Unreal Engine Marketplace listing](https://www.unrealengine.com/marketplace/en-US/product/sentry-01)
  - Version: [3.1.0](https://github.com/getsentry/sentry-unreal/blob/914169e0b4353fd5f55640bce3a49743fedeb8e2/plugin-dev/sentry-cli.properties#L1)
  - Observed CLI usage: [`debug-files upload`](https://github.com/getsentry/sentry-unreal/blob/914169e0b4353fd5f55640bce3a49743fedeb8e2/plugin-dev/Scripts/upload-debug-symbols.py#L216-L219).
- **.NET** — [getsentry/sentry-dotnet](https://github.com/getsentry/sentry-dotnet)
  - Package: [`Sentry` on NuGet](https://www.nuget.org/packages/Sentry)
  - Version: [2.58.2](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/Directory.Build.props#L102)
  - Observed CLI usage: [`debug-files upload`](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/src/Sentry/buildTransitive/Sentry.targets#L130), [`debug-files bundle-sources`](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/src/Sentry/buildTransitive/Sentry.targets#L256), [`upload-proguard`](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/src/Sentry/buildTransitive/Sentry.targets#L132), [`releases new`](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/src/Sentry/buildTransitive/Sentry.targets#L378), [`releases set-commits`](https://github.com/getsentry/sentry-dotnet/blob/f2ea5c496e8c747f3a742b681896222be9ee6512/src/Sentry/buildTransitive/Sentry.targets#L389).
- **Cordova** — [getsentry/sentry-cordova](https://github.com/getsentry/sentry-cordova)
  - Package: [`sentry-cordova`](https://www.npmjs.com/package/sentry-cordova)
  - Version: [2.43.1](https://github.com/getsentry/sentry-cordova/blob/f75effbc0897a4b80f9057a4acf3c603a26d8bec/yarn.lock#L1159)
  - Observed CLI usage: [`upload-dsym`](https://github.com/getsentry/sentry-cordova/blob/f75effbc0897a4b80f9057a4acf3c603a26d8bec/scripts/xcode-upload-debug-files.sh#L54).
- **React Native** — [getsentry/sentry-react-native](https://github.com/getsentry/sentry-react-native)
  - Package: [`@sentry/react-native`](https://www.npmjs.com/package/@sentry/react-native)
  - Version: [2.58.4](https://github.com/getsentry/sentry-react-native/blob/d63330ba47d42ae399a79b30daddb95a553ed841/packages/core/package.json#L73)
  - Observed CLI usage: [`debug-files upload`](https://github.com/getsentry/sentry-react-native/blob/d63330ba47d42ae399a79b30daddb95a553ed841/packages/core/scripts/sentry-xcode-debug-files.sh#L59), [`sourcemaps upload`](https://github.com/getsentry/sentry-react-native/blob/d63330ba47d42ae399a79b30daddb95a553ed841/packages/core/scripts/expo-upload-sourcemaps.js#L221), [`react-native gradle`](https://github.com/getsentry/sentry-react-native/blob/d63330ba47d42ae399a79b30daddb95a553ed841/packages/core/sentry.gradle#L174-L177).
- **JS bundler plugins** — [getsentry/sentry-javascript-bundler-plugins](https://github.com/getsentry/sentry-javascript-bundler-plugins)
  - Packages: [`@sentry/webpack-plugin`](https://www.npmjs.com/package/@sentry/webpack-plugin) ([package.json](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/webpack-plugin/package.json)), [`@sentry/vite-plugin`](https://www.npmjs.com/package/@sentry/vite-plugin) ([package.json](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/vite-plugin/package.json)), [`@sentry/rollup-plugin`](https://www.npmjs.com/package/@sentry/rollup-plugin) ([package.json](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/rollup-plugin/package.json)), [`@sentry/esbuild-plugin`](https://www.npmjs.com/package/@sentry/esbuild-plugin) ([package.json](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/esbuild-plugin/package.json))
  - Version: [2.57.0](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/yarn.lock#L2769)
  - Observed CLI usage (JS API): [`SentryCli.releases.new`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L475), [`SentryCli.releases.uploadSourceMaps`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L493-L501), [`SentryCli.releases.setCommits`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L503-L509), [`SentryCli.releases.finalize`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L528-L529), [`SentryCli.releases.newDeploy`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L533), [`SentryCli.execute(['sourcemaps','inject',…])`](https://github.com/getsentry/sentry-javascript-bundler-plugins/blob/0131e8674fd8cfc7b8cf00acfe74277a9767ebfb/packages/bundler-plugin-core/src/build-plugin-manager.ts#L557-L560).
- **Wizard/setup tooling** — [getsentry/sentry-wizard](https://github.com/getsentry/sentry-wizard)
  - Package: [`@sentry/wizard`](https://www.npmjs.com/package/@sentry/wizard)
  - Version: [@sentry/cli@^2](https://github.com/getsentry/sentry-wizard/blob/06e8f639a4495951e6539ca57903bfb544993458/src/sourcemaps/tools/sentry-cli.ts#L37-L39)
  - Observed CLI usage: [`sourcemaps inject`](https://github.com/getsentry/sentry-wizard/blob/06e8f639a4495951e6539ca57903bfb544993458/src/sourcemaps/tools/sentry-cli.ts#L150-L156), [`sourcemaps upload`](https://github.com/getsentry/sentry-wizard/blob/06e8f639a4495951e6539ca57903bfb544993458/src/sourcemaps/tools/sentry-cli.ts#L150-L156).
- **GitHub Action** — [getsentry/action-release](https://github.com/getsentry/action-release)
  - Package: [Sentry Release GitHub Action](https://github.com/getsentry/action-release)
  - Version: [2.58.4](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/yarn.lock#L1049)
  - Observed CLI usage (JS API): [`SentryCli.releases.new`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L46), [`SentryCli.releases.setCommits`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L55-L67), [`SentryCli.execute(['sourcemaps','inject',…])`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L85), [`SentryCli.releases.uploadSourceMaps`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L106), [`SentryCli.releases.newDeploy`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L120), [`SentryCli.releases.finalize`](https://github.com/getsentry/action-release/blob/f86218f1105cbe0847f30d0649d3c1789732ca97/src/main.ts#L132).

## Third‑party redistributions

This is a non-exhaustive lists of Sentry CLI redistributions maintained by third parties.

- **Scoop (Windows)** — third‑party bucket entry referenced in docs ([Sentry CLI install via Scoop](https://docs.sentry.io/cli/installation/#installation-via-scoop); [`sentry-cli` on Scoop](https://scoop.sh/#/apps?q=sentry-cli); [`sentry-cli` Scoop manifest](https://github.com/ScoopInstaller/Main/blob/2605886d157b61413b82118ec378253eaa3a8211/bucket/sentry-cli.json)).
- **Homebrew (main)** — third‑party formula (not referenced in our docs) ([`sentry-cli` on Homebrew Formulae](https://formulae.brew.sh/formula/sentry-cli); [`sentry-cli` formula in homebrew-core](https://github.com/Homebrew/homebrew-core/blob/6b5956167aaaa3bec4eb301df7d58c1261f69120/Formula/s/sentry-cli.rb)).

## Notes on scope

- Coverage includes first‑party distribution mechanisms in `getsentry/sentry-cli`, installer repos referenced by the official docs, explicit third‑party redistributions (Scoop and Homebrew main), and first‑party consumers that bundle or download the CLI.
- Links are permalinks to the latest commit on each repo as of 2026‑01‑29.
