# Sentry Commit Message Enforcement Implementation Plan

## Background & Motivation

This plan implements enforcement of [Sentry's commit message guidelines](https://develop.sentry.dev/engineering-practices/commit-messages/) in the sentry-cli repository.

### Problem Statement

- Sentry has established commit message conventions but no enforcement mechanism
- Most commits are squash-merged in GitHub, allowing developers to modify commit messages during merge
- Need to ensure both individual commits in PRs and final squash-merge commits follow the standard
- Want to catch violations early (locally) but also enforce at the CI level

### Key Requirements

- **Local enforcement**: Block commits locally with option to override (`--no-verify`)
- **CI enforcement**: Mandatory validation that blocks merges
- **Dual validation**: Validate both individual PR commits and final squash-merge commits
- **Minimal dependencies**: Avoid introducing heavy dependencies, leverage existing toolchain
- **Rust project focus**: Solution appropriate for a primarily Rust codebase

## Sentry Commit Message Format

Based on the [official guidelines](https://develop.sentry.dev/engineering-practices/commit-messages/):

```
<type>(<scope>): <subject>
<BLANK LINE>
<body>
<BLANK LINE>
<footer>
```

### Validation Rules (Subject Line Focus)

- **Type**: Must be one of: `build`, `ci`, `docs`, `feat`, `fix`, `perf`, `ref`, `style`, `test`, `meta`, `license`, `revert`
- **Scope**: Optional, lowercase if present
- **Subject**:
  - Capitalize first letter
  - Use imperative mood ("Add" not "Added")
  - No trailing period
  - Combined header max 70 characters
- **Body/Footer**: Not validated initially (future enhancement)

### Explicit Non-Requirements

- **No JIRA validation**: sentry-cli uses GitHub/Linear, not Jira
- **No existing history validation**: Only enforce going forward
- **Subject line focus**: Body/footer validation deferred
- **Perfect squash merge enforcement**: GitHub doesn't provide native controls to prevent commit message editing during squash merge

## Implementation Plan

### Phase 1: Local Git Hooks with pre-commit

#### 1.1 pre-commit Configuration

- **File**: `.pre-commit-config.yaml`
- **Framework**: Use [pre-commit](https://pre-commit.com/) for hook management
- **Benefits**:
  - Automatic hook installation via `pre-commit install`
  - Language-agnostic framework with existing ecosystem
  - Built-in support for bypassing hooks (`--no-verify`)
  - Handles hook lifecycle and updates automatically
  - Can run same hooks locally and in CI

#### 1.2 Commit Message Hook Implementation

- **Primary approach**: Use existing pre-commit hook if configurable for Sentry format
- **Fallback approach**: Create custom hook if existing hooks don't support Sentry requirements
- **Hook candidates**: `conventional-pre-commit`, `gitlint`, or similar commit message validators
- **Custom implementation**:
  - **File**: `scripts/validate-commit-msg.py`
  - **Function**: Sentry-specific commit message validation
  - **Features**: Validates type enum, subject format, capitalization, and length limits
- **Configuration**: Use `stages: [commit-msg]` in `.pre-commit-config.yaml`

#### 1.3 Developer Setup

- **Installation**: `pre-commit install` (single command setup)
- **Documentation**: Clear instructions in CONTRIBUTING.md
- **Template**: Optional `.gitmessage` file with examples
- **Integration**: Consider adding to build scripts or onboarding docs

### Phase 2: CI/CD Integration

#### 2.1 CI Validation with pre-commit

- **Implementation**: Use `pre-commit run --all-files` in GitHub Actions
- **File**: Integrate into existing `.github/workflows/ci.yml`
- **Validation Logic**:
  - **For PRs**: Validate all individual commit messages in the PR
  - **Triggers**: Only on `pull_request` events (NOT on push to main branches)
  - **Rebase merges**: Individual commits already validated in PR before merge ✅
  - **Squash merges**: ⚠️ **Cannot be fully enforced** - GitHub allows editing commit message during merge, see section 2.3.
- **Features**:
  - Same hooks run locally and in CI for consistency
  - Leverages pre-commit's caching for performance
  - Validates all files changed in PR using same pre-commit config
- **Branch Protection**: Require this check to pass before allowing merge

#### 2.2 Main CI Integration

- **File**: `.github/workflows/ci.yml` (modify existing)
- **Implementation**: Add dedicated `pre_commit` job
- **Changes**:
  - Add new job: `pre_commit` running `pre-commit run --all-files`
  - Include caching for pre-commit environments
  - Update `required` job dependencies to include pre-commit validation

**GitHub Actions Implementation**:

```yaml
- name: Set up Python
  uses: actions/setup-python@v4
  with:
    python-version: '3.x'
- name: Install pre-commit
  run: pip install pre-commit
- name: Run pre-commit
  run: pre-commit run --all-files
```

#### 2.3 Squash Merge Limitation

**Limitation**: GitHub does not provide a way to prevent editing commit messages during squash merge, making full enforcement impossible for squash merges.

**Accepted Approach**: Keep squash merging enabled with current limitations

- **Individual commits**: Fully validated in PRs before merge
- **Final squash commit**: Can be edited during merge (partial enforcement)
- **Mitigation strategy**: Strong local git hooks + team education
- **Developer experience**: Maintains current workflow with minimal changes

#### 2.4 Repository Settings (Required)

- **Branch Protection Rules**: Must be configured for main branches (`master`, `1.x`, `release/**`)
  - Require status checks to pass before merging
  - Require "Check required jobs" status check (includes commit message validation)
  - Restrict pushes to these branches (force use of PRs)
- **Merge Settings**:
  - Rebase merging: ✅ Full validation possible
  - Squash merging: ⚠️ Partial validation only (see limitation above)

### Phase 3: Documentation & Developer Experience

#### 3.1 Documentation Updates

- **File**: `CONTRIBUTING.md` (update existing)
- **Add section**: Commit message guidelines and setup instructions
- **Reference**: Link to official Sentry commit guidelines
- **Setup guidance**: Instructions for hook installation

#### 3.2 Developer Onboarding

- **Automatic notification**: Inform developers about git hooks during repo setup
- **Clear instructions**: Make setup as frictionless as possible
- **Fallback documentation**: Ensure CONTRIBUTING.md has complete setup info

## Technical Decisions Made

### Hook Management: pre-commit Framework

- **Decision**: Use pre-commit framework for git hook management
- **Rationale**:
  - Mature, widely-adopted framework for multi-language hook management
  - Eliminates need for custom hook installation scripts
  - Consistent local and CI execution
  - Large ecosystem of existing hooks
  - Handles hook lifecycle, updates, and caching automatically
- **Benefits**:
  - Simplified setup (`pre-commit install`)
  - Same hooks run locally and in CI
  - Industry standard approach
  - Better developer experience

### Enforcement Strategy

- **Local**: Block commits with override option (`--no-verify`)
- **CI**: Mandatory enforcement via required status checks on PRs
- **Branch Protection**: Essential to prevent direct pushes and enforce PR-based workflow
- **Scope**:
  - Individual commits in PRs (works for both rebase and squash merge workflows)
  - ⚠️ **Limitation**: Final squash merge commit message can be edited and bypass validation
- **Key Insights**:
  - GitHub Actions on `push` events run too late to prevent bad commits
  - GitHub provides no native way to prevent commit message editing during squash merge
  - Enforcement must happen during PR validation for maximum coverage

### Integration Approach

- **CI Structure**: Extract validation to separate workflow, call from main ci.yml
- **Consistency**: Follows existing pattern with lint.yml and test.yml
- **Required Jobs**: Integrate with existing "Check required jobs" pattern

### Validation Scope

- **Focus**: Subject line validation only initially
- **Future**: Body/footer validation can be added later
- **History**: No retroactive validation of existing commits

## Future Enhancements

- **Advanced message validation**: Body and footer format checking
- **Imperative mood detection**: More sophisticated language analysis
- **Developer tooling**: Suggest fixes for invalid commit messages
- **Integration features**: Linear ticket references, automated changelog generation
- **Compliance metrics**: Track and report on commit message adherence
- **Full enforcement options**: Consider merge queue or disable squash merging if perfect enforcement becomes required
