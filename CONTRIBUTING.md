# Contributing to Sentry CLI

> [!IMPORTANT]
> Sentry CLI is no longer under active development, as the [new Sentry CLI](https://cli.sentry.dev/) is replacing it. Please contribute to the new CLI in [`getsentry/toolkit`](https://github.com/getsentry/toolkit/tree/main/packages/cli) instead.

We only accept pull requests that fix a bug in this CLI in one of these cases:

- A recent Sentry CLI release introduced a serious regression.
- A bug or limitation in the new CLI is a fundamental blocker that prevents you from adopting it, so you need the fix in this CLI. This should be exceptionally rare, as most such blockers should be [reported](https://github.com/getsentry/toolkit/issues/new) and fixed in the new CLI.

Very few changes meet this bar.

Before opening a pull request, open a [bug report](https://github.com/getsentry/sentry-cli/issues/new?template=02-bug.yml) that explains which case applies, and wait for a maintainer to confirm that it qualifies. We will close all other pull requests.

This policy does not apply to maintenance work by Sentry CLI maintainers.
