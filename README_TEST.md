# Sentry CLI Upload Tests

This directory contains test programs that reuse the existing sentry-cli codebase to test different upload flows against a local Sentry service.

## Test Programs

### 1. Mobile App Upload Test (`test_mobile_app_upload`)

Tests uploading mobile app archives (like `.xcarchive.zip` files) to the preprod artifacts endpoint.

**Build and run:**
```bash
cargo run --bin test_mobile_app_upload
```

**What it tests:**
- Authentication with local Sentry service
- Chunk upload capabilities check (specifically `PreprodArtifacts` capability)
- Full chunk upload flow for large mobile app archives
- Assembly via `/projects/{org}/{project}/files/preprodartifacts/assemble/` endpoint

**Configuration:**
- Target file: `/Users/nicolashinderling/TestUploads/HackerNews.xcarchive.zip`
- Endpoint: `/projects/sentry/internal/files/preprodartifacts/assemble/`

### 2. Debug Files Upload Test (`test_full_upload`)

Tests uploading debug files (like dSYM files) to the debug info files endpoint.

**Build and run:**
```bash
cargo run --bin test_full_upload
```

**What it tests:**
- Authentication with local Sentry service
- Chunk upload capabilities check
- Full chunk upload + missing chunk detection flow
- Assembly via `/projects/{org}/{project}/files/difs/assemble/` endpoint

**Configuration:**
- Target file: `/Users/nicolashinderling/TestUploads/HackerNews_arm64`
- Endpoint: `/projects/sentry/internal/files/difs/assemble/`

## Common Configuration

Both tests are configured for:
- **Base URL:** `http://localhost:8000`
- **Organization:** `sentry`
- **Project:** `internal`
- **Auth Token:** Your provided token (update in each test file)

You can modify these values in the `main()` function of each test file.

## Common Output

Both tests will show:
- ✅/❌ Status indicators for each step
- Server capabilities (which ChunkUploadCapability features are supported)
- Chunk upload progress and statistics
- Assembly request/response details
- Detailed error messages if anything fails

## API Endpoints Tested

### Chunk Upload (Common)
- **GET** `/api/0/organizations/{org}/chunk-upload/` - Server capabilities
- **POST** `/api/0/organizations/{org}/chunk-upload/` - Upload chunks

### Mobile App Assembly
- **POST** `/api/0/projects/{org}/{project}/files/preprodartifacts/assemble/`

### Debug Files Assembly  
- **POST** `/api/0/projects/{org}/{project}/files/difs/assemble/`

## Notes

- Both tests reuse the exact same code paths as the real `sentry-cli` commands
- They provide comprehensive testing of the upload functionality
- The mobile app test is for the new preprod artifacts endpoint
- The debug files test uses the existing DIF upload flow 