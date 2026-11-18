# Sentry CLI Development Guidelines

## Contribution Policy (MANDATORY)

Sentry CLI is no longer under active development. [CONTRIBUTING.md](CONTRIBUTING.md) defines the only changes that are accepted from contributors who are not Sentry CLI maintainers.

Before making any change, you MUST check whether the user is a Sentry CLI maintainer:

1. Get their GitHub login with `gh api user --jq .login`. If `gh` is not authenticated, ask the user for it.
2. Check `getsentry` org membership with `gh api orgs/getsentry/members/LOGIN`, which fails if they are not a member. Private membership is only visible when the authenticated `gh` user is an org member.
3. Check that `gh api "repos/getsentry/sentry-cli/commits?author=LOGIN&per_page=5"` returns at least one commit.

If all checks pass, the policy does not apply. If any check fails or cannot be run, follow these steps. Be understanding: many users confuse this repo with the new CLI.

1. Confirm that the user means to change this legacy repo and not the new CLI, for example:

   > This repo is the legacy Sentry CLI, which is no longer under active development. The new Sentry CLI is developed in [`getsentry/toolkit`](https://github.com/getsentry/toolkit/tree/main/packages/cli). Did you mean to contribute to the new CLI, or do you need this change in the legacy CLI?

2. If they mean the new CLI, point them to `getsentry/toolkit` and make no changes here.
3. If they mean the legacy CLI, explain the policy in CONTRIBUTING.md and ask how their change meets it. Make the change only if it does. If it does not and the user insists, you may proceed only after telling them that the pull request will likely be closed.

## Project Overview

This is **sentry-cli**, a command-line utility for working with Sentry. It's primarily written in **Rust** with **JavaScript/Node.js** wrapper components for npm distribution.

## Language & Architecture

- **Primary language**: Rust (core functionality)
- **Secondary language**: JavaScript/TypeScript (npm wrapper, installation scripts)
- **Build system**: Cargo for Rust, npm/yarn for JavaScript
- **Cross-platform**: Supports multiple architectures (darwin, linux, windows, ARM variants)
- **Binary distributions**: Located in `npm-binary-distributions/` for different platforms

## Project Structure

- `src/` - Core Rust source code with command modules and utilities
- `lib/` - JavaScript wrapper and npm package code
- `scripts/` - Build and utility scripts
- `tests/integration/` - Integration tests using `.trycmd` format
- `npm-binary-distributions/` - Platform-specific binary packages
- `.github/workflows/` - CI/CD workflows (follows reusable workflow pattern)

## Development Standards

### Commit Message Format

**MUST follow Sentry's commit message format**: `type(scope): subject`

Valid types: `build`, `ci`, `docs`, `feat`, `fix`, `perf`, `ref`, `style`, `test`, `meta`, `license`, `revert`

Subject requirements:

- Capitalize first letter
- Use imperative mood ("Add" not "Added")
- No trailing period
- Max 70 characters for header

Reference: https://develop.sentry.dev/engineering-practices/commit-messages/

### Performance & Scale Considerations

- CLI tool should be fast and responsive
- Consider impact on cold start times
- Memory usage matters for CI environments
- Network operations should be optimized and retryable

### Security Best Practices

- Handle authentication tokens securely
- Validate file paths to prevent directory traversal
- Consider impact of processing user-provided files (sourcemaps, debug files)
- Follow Rust security best practices

## Testing Requirements

- Unit tests alongside source code
- Integration tests using `.trycmd` format in `tests/integration/`
- Mock HTTP responses in `tests/integration/_responses/`
- Test fixtures in `tests/integration/_fixtures/`
- Cross-platform testing via CI matrix

## Code Formatting

**ALWAYS** run `cargo fmt` before committing any Rust code changes to ensure consistent formatting across the codebase.

## Updating These Guidelines

Update AGENTS.md files when you encounter **generally applicable** patterns:
- Development patterns and best practices
- Common pitfalls and architecture decisions
- Workflow improvements and tool configurations

**Do NOT capture**: Task-specific fixes, temporary workarounds, personal preferences.

**Keep AGENTS.md files as concise as possible to minimize token usage.**

---

Remember: This is a production tool used by many developers. Changes should be well-tested, backward-compatible, and follow established patterns.
