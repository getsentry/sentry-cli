# Migration Plan: Clap Builder to Derive Pattern

## Overview

Migrate sentry-cli's 60+ commands from clap builder pattern to clap derive pattern, incrementally over multiple PRs.

## Architecture Decisions

1. **Keep re-parsing approach** - Commands use `SentryCLI::parse()` in `execute()` to get typed args. This is simpler than `from_arg_matches()` and performance is irrelevant (microseconds vs network calls).

2. **Accept temporary global args duplication** - Both builder `app()` and derive `SentryCLI` define globals during transition. Duplication eliminated post-migration.

3. **Use existing Config pattern** - Derive commands use `Config::current().get_org_and_project_defaults()` for fallbacks, with inline resolution (same pattern as `logs/list.rs`).

4. **Keep `--url` non-global** - Matches current behavior, simplifies migration.

5. **Dynamic hiding via `make_command`** - Commands with runtime visibility (`uninstall`, `update`) keep `make_command` functions for `.hide()` logic since derive attributes must be const.

## PR Dependencies

**Structure:** PRs are **logically independent** but share `derive_parser.rs`.

- Every migration PR adds variants to `SentryCLICommand` enum in `derive_parser.rs`
- PRs can be **developed and reviewed in parallel**
- PRs should be **merged sequentially** to avoid trivial merge conflicts
- Within a tier, PRs can be merged in any order
- Cleanup PR must come after all migrations complete

**Workflow:**
1. Open multiple PRs in parallel for review
2. Merge one at a time
3. Rebase remaining PRs before their merge
4. Alternatively: use a single long-running branch and stack PRs on top of each other

## Migration Tiers

### Already Done
- `logs`
- `dart-symbol-map`

### Tier 1 - Simple Standalone (1 PR)
Commands validating infrastructure, no org/project:
- `completions` - Validates derive structure generates correct Command tree
- `info` - Simple flags, uses defaults pattern
- `uninstall` - Simple, cfg-gated, dynamic hiding
- `update` - Simple, cfg-gated, dynamic hiding

### Tier 2 - Org/Project Commands (3 PRs)
- **PR 2a**: `login`, `organizations`, `projects`
- **PR 2b**: `repos`, `events`, `issues`
- **PR 2c**: `monitors`, `send-envelope`, `send-event`, `bash-hook`

### Tier 3 - Release-Related (3 PRs)
- **PR 3a**: `deploys`, `build`
- **PR 3b**: `upload-dsym`, `upload-dif` (deprecated wrappers)
- **PR 3c**: `upload-proguard`

### Tier 4 - Complex Commands (4 PRs, 1 each)
- `releases` - 9 subcommands + legacy `releases deploys <VERSION>` backward compat
- `sourcemaps` - Complex upload, org/project/release at parent
- `debug-files` - 6 subcommands
- `react-native` - xcode/gradle workflows

### Cleanup Phase (Final PR)
1. Remove `each_subcommand!` macro from `mod.rs`
2. Simplify `add_commands()` to single `SentryCLI::command()` call
3. Replace `run_command()` with derive-based dispatch
4. Remove `ArgExt` trait from `utils/args.rs` (keep validators)
5. Make `SentryCLI` struct the canonical CLI definition

## Command Migration Pattern

For each command:

```rust
// 1. Define args struct with derive
#[derive(Args)]
pub struct MyCommandArgs {
    #[arg(short = 'o', long = "org")]
    org: Option<String>,

    #[arg(short = 'p', long = "project")]
    project: Option<String>,

    #[command(subcommand)]
    subcommand: MySubcommand,
}

// 2. Add to SentryCLICommand enum in derive_parser.rs
#[derive(Subcommand)]
pub(super) enum SentryCLICommand {
    // ... existing
    MyCommand(MyCommandArgs),
}

// 3. Keep make_command for help text (uses augment_subcommands)
pub(super) fn make_command(command: Command) -> Command {
    MySubcommand::augment_subcommands(command.about("..."))
}

// 4. Update execute to use typed args
pub(super) fn execute(_: &ArgMatches) -> Result<()> {
    let SentryCLICommand::MyCommand(args) = SentryCLI::parse().command else {
        unreachable!();
    };

    // Resolve org/project with defaults
    let config = Config::current();
    let (default_org, default_project) = config.get_org_and_project_defaults();
    let org = args.org.or(default_org).ok_or_else(||
        format_err!("An organization ID or slug is required (provide with --org)")
    )?;

    // Dispatch to implementation
    match args.subcommand {
        MySubcommand::Foo(foo_args) => foo::execute(&org, foo_args),
    }
}
```

## Special Cases

### Feature-Gated Commands (`uninstall`, `update`)
```rust
#[derive(Subcommand)]
pub(super) enum SentryCLICommand {
    #[cfg(not(feature = "managed"))]
    Uninstall(UninstallArgs),
    #[cfg(not(feature = "managed"))]
    Update(UpdateArgs),
}
```

### Dynamic Hiding
Keep `make_command` for runtime conditions:
```rust
pub(super) fn make_command(mut command: Command) -> Command {
    if !can_update_sentrycli() {
        command = command.hide(true);
    }
    command
}
```

### `releases deploys <VERSION>` Backward Compat
```rust
#[derive(Subcommand)]
enum ReleasesSubcommand {
    // ... normal subcommands

    #[command(hide = true)]
    Deploys(DeploysLegacyArgs),  // Has version: String field + DeploysSubcommand
}
```

## Success Criteria

### Per-Command
- `--help` output identical to builder version
- All existing `.trycmd` integration tests pass
- Shell completions include command and all arguments
- Error messages for validation identical

### Overall Migration
- `each_subcommand!` macro removed
- All commands in `SentryCLICommand` enum
- No `ArgMatches` in command execute functions
- All CI tests pass

## Critical Files

- `src/commands/derive_parser.rs` - Central `SentryCLI` struct and `SentryCLICommand` enum
- `src/commands/mod.rs` - Entry point, `app()`, `add_commands()`, `run_command()`
- `src/commands/logs/list.rs` - Reference pattern for org/project resolution
- `src/utils/args.rs` - `ArgExt` trait (to deprecate) and validators (to keep)
- `tests/integration/_cases/help/help.trycmd` - Help output golden file

## Notes

- **`send-metric` dead code** - Files exist at `src/commands/send_metric/` but not integrated into `mod.rs`. Ignoring for now - not part of this migration scope.

## Verification Steps

After each command migration:
1. Run `cargo test` - all unit tests pass
2. Run `cargo run -- <command> --help` - compare to previous output
3. Run integration tests: `cargo test --test integration`
4. Test shell completions: `cargo run -- completions bash | grep <command>`
