# Contributing to Sentry CLI

> [!IMPORTANT]
> Sentry CLI is no longer under active development, as the [new Sentry CLI](https://cli.sentry.dev/) is replacing it. Please contribute to the new CLI in [`getsentry/toolkit`](https://github.com/getsentry/toolkit/tree/main/packages/cli) instead.

We only accept pull requests that fix a bug in this CLI in one of these cases:

- A recent Sentry CLI release introduced a serious regression.
- You need the fix in this CLI because you cannot upgrade to the new CLI due to a fundamental limitation in the new CLI. In this case, please first [report](https://github.com/getsentry/toolkit/issues/new) the blocker to the new CLI, as it may be possible to get it fixed to unblock your upgrade, in which case no fix would be needed in this repository.

Before opening a pull request, open a [bug report](https://github.com/getsentry/sentry-cli/issues/new?template=02-bug.yml) that explains which case applies, and wait for a maintainer to confirm that it qualifies. We will close all other pull requests.

This policy does not apply to maintenance work by Sentry CLI maintainers.

## Security issues

Please report security issues as described in our [security policy](https://github.com/getsentry/sentry-cli/security/policy). Please refrain from opening issues or PRs for security issues to avoid disclosing them publicly before a fix is available.
