# Plan: Refactor Linux Build CI to Use Native Container Support

## Overview

This plan outlines the refactoring of the Linux build job in `.github/workflows/build.yml` to eliminate the `scripts/build-in-docker.sh` script and instead use GitHub Actions' native container support via the `jobs.<job>.container` field.

**Key objectives:**

- Replace Docker script with native container support
- Remove legacy smoke test that relies on Travis CI environment variables
- Pin all GitHub Actions to commit hashes for enhanced security

## Current State Analysis

### What the build-in-docker.sh script does:

1. Sets up Docker image: `messense/rust-musl-cross:${DOCKER_TAG}`
2. Mounts volumes:
   - Current directory as read-only at `/work`
   - `target` directory for build output
   - Cargo registry cache from `$HOME/.cargo/registry`
3. Runs `cargo build --release --target=${TARGET} --locked`
4. Optionally runs a smoke test (only for non-PR builds from getsentry/sentry-cli) - **Note: This uses Travis CI environment variables and never executes in GitHub Actions**
5. Fixes permissions on shared directories after Docker run

### Current workflow structure:

- Runs on `ubuntu-24.04` runner
- Uses a matrix for different architectures (i686, x86_64, armv7, aarch64)
- Calls the shell script with `TARGET` and `DOCKER_TAG` environment variables

## Refactoring Steps

### 1. Update the Linux job to use container field

Replace the current Linux job structure with:

```yaml
linux:
  strategy:
    fail-fast: false
    matrix:
      include:
        - arch: i686
          target: i686-unknown-linux-musl
          container: i686-musl
        - arch: x86_64
          target: x86_64-unknown-linux-musl
          container: x86_64-musl
        - arch: armv7
          target: armv7-unknown-linux-musleabi
          container: armv7-musleabi
        - arch: aarch64
          target: aarch64-unknown-linux-musl
          container: aarch64-musl

  name: Linux ${{ matrix.arch }}
  runs-on: ubuntu-24.04

  container:
    image: messense/rust-musl-cross:${{ matrix.container }}
    options: --user root
```

### 2. Convert build steps

Replace the "Build in Docker" step with direct commands:

```yaml
steps:
  - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # 5.0.0

  - name: Build
    run: cargo build --release --target=${{ matrix.target }} --locked

  - name: Rename Binary
    run: mv target/${{ matrix.target }}/release/sentry-cli sentry-cli-Linux-${{ matrix.arch }}

  - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # 4.6.2
    with:
      name: artifact-bin-linux-${{ matrix.arch }}
      path: sentry-cli-Linux-${{ matrix.arch }}
      if-no-files-found: 'error'
```

### 3. Handle volume mounts and caching

GitHub Actions automatically handles:

- The workspace is mounted at `/github/workspace` (or can use relative paths)
- The checkout action handles the workspace setup
- No need for explicit volume mounts

For Cargo cache, add a caching step:

```yaml
- name: Cache cargo registry
  uses: actions/cache@1a9e2138d905efd099035b49d8b7a3888c653ca8 # v4.0.2
  with:
    path: |
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
    key: ${{ runner.os }}-cargo-${{ matrix.target }}-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-${{ matrix.target }}-
      ${{ runner.os }}-cargo-
```

### 4. Remove the build-in-docker.sh script

After verifying the refactored workflow works correctly:

1. Delete `scripts/build-in-docker.sh`
2. Update any documentation that references this script

## Key Differences and Benefits

### What changes:

1. **No Docker command execution**: GitHub Actions handles container lifecycle
2. **No permission fixes needed**: Container runs with proper permissions by default
3. **Removed legacy smoke test**: The smoke test relied on Travis CI environment variables and never executed in GitHub Actions
4. **Better caching**: Native GitHub Actions cache instead of Docker volume mounts
5. **Cleaner logs**: Direct command output instead of Docker wrapper output

### Benefits:

1. **Reduced complexity**: No separate shell script to maintain
2. **Better integration**: Native GitHub Actions features (caching, conditionals)
3. **Improved debugging**: Direct access to build logs without Docker wrapper
4. **Faster builds**: Better cache utilization with GitHub Actions cache
5. **Consistency**: Similar pattern to macOS and Windows jobs
6. **Enhanced security**: Actions pinned by commit hash for better supply chain security
7. **Cleaner codebase**: Removal of legacy Travis CI code

## Potential Challenges

1. **Container user permissions**: May need to adjust the `--user` option in container options
2. **Working directory**: Ensure paths work correctly within the container context
3. **Environment variables**: Verify all necessary env vars are passed through
4. **Cache permissions**: Ensure cache directories are accessible within container

## Testing Strategy

1. Create a test branch with the refactored workflow
2. Run builds for all architectures
3. Verify:
   - Build completes successfully
   - Binaries are correctly produced
   - Artifacts are uploaded correctly
   - Cache is utilized effectively
4. Compare build times with current approach
5. Test on both push and pull_request events

## Migration Steps

1. **Phase 1**: Create new workflow file for testing alongside existing one
2. **Phase 2**: Run both workflows in parallel for a few builds to compare
3. **Phase 3**: Switch to new workflow, keep old script for rollback
4. **Phase 4**: After stable operation, remove old script

## Alternative Considerations

If native container support has limitations:

1. Consider using composite actions to encapsulate build logic
2. Evaluate if setup-rust action with cross-compilation is viable
3. Consider using buildx for multi-platform builds (though more complex)

## Success Criteria

- All Linux architectures build successfully
- Build times are similar or improved
- No regression in binary functionality
- Legacy Travis CI code removed
- Simplified maintenance burden
