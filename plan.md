# Automated Docker Image Hash Updates Plan

## Problem Statement

The `build.yml` workflow uses hardcoded container SHA hashes for `ghcr.io/rust-cross/rust-musl-cross` images. These hashes become stale when the upstream images are updated, requiring manual updates to stay current with security patches and toolchain improvements.

## Proposed Solution

### 1. Extract Container Configuration

Create `.github/rust-cross-containers.yml`:

```yaml
# Container configurations for rust-musl-cross builds
containers:
  - target: i686-unknown-linux-musl
    arch: i686
    image_tag: i686-unknown-linux-musl
    current_sha: 5539b836c671353becda5c18f7675138a8724c5a4d3ecf5c26ba74e40239f89a
  - target: x86_64-unknown-linux-musl
    arch: x86_64
    image_tag: x86_64-unknown-linux-musl
    current_sha: 8da7503d1199ddea00be75ef56d971da79f023fa4b7a8366be3df24fd31c2279
  - target: armv7-unknown-linux-musleabi
    arch: armv7
    image_tag: armv7-unknown-linux-musleabi
    current_sha: 5c9b9ee4777ad74dcee6913171918b6d02bcaab25289d751912a27e151af0224
  - target: aarch64-unknown-linux-musl
    arch: aarch64
    image_tag: aarch64-unknown-linux-musl
    current_sha: 8098ec3ebd8268a8ae37ef3a5bc35def863dda30a7f6817b65ce70ecad4bb413
```

### 2. Create Update Automation Workflow

Create `.github/workflows/update-rust-cross-hashes.yml`:

**Trigger**: Scheduled (cron) - runs 1st and 15th of each month
**Permissions**: Read contents, write PRs
**Steps**:

1. Checkout repository
2. Install required tools (jq, curl)
3. Run update script to check for new hashes
4. If changes detected, create PR with detailed information

### 3. Hash Update Script

Create `scripts/update-rust-cross-hashes.js` (or `.sh`):

**Responsibilities**:

- Query GitHub Container Registry API for each image tag
- Get current image digest (SHA256 hash)
- Compare with stored hashes in configuration file
- Update configuration file if changes detected
- Generate commit message and PR body with details

**API Endpoints**:

- Use GitHub Container Registry API: `GET /v2/{name}/manifests/{reference}`
- Extract `docker-content-digest` header for SHA256 hash

### 4. Modify Build Workflow

Update `build.yml` to read from configuration file:

**Strategy**:

- Use `fromJSON()` to parse the YAML configuration
- Replace hardcoded matrix with dynamic matrix from config file
- Maintain same job structure and steps

### 5. PR Content Enhancement

**PR Title**: `chore: Update rust-musl-cross container hashes`

**PR Body Template**:

```markdown
## Container Hash Updates

This PR updates the container SHA hashes for rust-musl-cross images.

### Changes Detected

| Target   | Previous Hash | New Hash     |
| -------- | ------------- | ------------ |
| {target} | `{old_hash}`  | `{new_hash}` |

### Upstream Changes

- [View commits since last update](link-to-ghcr-commits)
- [rust-cross/rust-musl-cross repository](https://github.com/rust-cross/rust-musl-cross)

### Verification

- [ ] All builds pass with new hashes
- [ ] No breaking changes detected
- [ ] Security scan results reviewed

_This PR was automatically generated on {date}_
```

## Implementation Steps

### Step 1: Configuration Extraction

1. Create `.github/rust-cross-containers.yml` with current mappings
2. Add validation schema/comments for maintainability

### Step 2: Build Workflow Migration

1. Modify `build.yml` matrix to read from config file
2. Test that builds work identically with new structure
3. Ensure no behavioral changes in the build process

### Step 3: Update Script Development

1. Create `scripts/update-rust-cross-hashes.js`
2. Implement registry API queries
3. Add hash comparison and file update logic
4. Include error handling and logging

### Step 4: Automation Workflow

1. Create `.github/workflows/update-rust-cross-hashes.yml`
2. Implement scheduled trigger (1st and 15th monthly)
3. Add PR creation with rich content
4. Include failure notification mechanisms

### Step 5: Integration & Testing

1. Test the full automation flow in a fork/branch
2. Verify PR creation works correctly
3. Validate that generated PRs build successfully
4. Document the new process for maintainers

## Technical Considerations

### Registry API Access

- Use GitHub Container Registry API (no auth needed for public images)
- Handle rate limiting gracefully
- Include retry logic for network failures

### Security

- Pin all GitHub Actions by commit hash (following project convention)
- Use minimal permissions for automation workflow
- Validate SHA256 hashes properly

### Error Handling

- Graceful failure when registry is unavailable
- Clear error messages for debugging
- Skip updates if unable to verify new hashes

### Maintenance

- Configuration file should be well-documented
- Include instructions for manual updates if needed
- Consider adding dry-run mode for testing

## Benefits

1. **Automated Security**: Stay current with upstream security patches
2. **Reduced Maintenance**: No more manual hash updates
3. **Transparency**: PRs provide visibility into what's changing
4. **Rollback Safety**: Easy to revert if issues discovered
5. **Extensibility**: Easy to add new targets or modify schedule

## Risks & Mitigations

- **Registry API changes**: Monitor for API deprecations, include error handling
- **Breaking image updates**: PR review process catches issues before merge
- **Schedule conflicts**: Choose times that don't conflict with releases
- **False positives**: Robust hash comparison prevents unnecessary PRs

## Future Enhancements

1. **Smart scheduling**: Check for updates more frequently if pattern of releases detected
2. **Changelog integration**: Parse upstream commit messages for impact analysis
3. **Dependency tracking**: Correlate with Rust toolchain updates
4. **Health monitoring**: Track update success/failure rates over time
