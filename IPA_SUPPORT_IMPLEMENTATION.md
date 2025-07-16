# iOS IPA File Support Implementation

This document summarizes the implementation of iOS .ipa file support in the sentry-cli mobile_app upload command.

## Overview

The mobile_app command now supports uploading iOS .ipa files in addition to the existing support for .apk, .aab, and .xcarchive files. When an .ipa file is provided, it is automatically converted to an xcarchive structure before uploading, allowing it to use the existing xcarchive upload infrastructure.

## Changes Made

### 1. File Format Detection (`src/utils/mobile_app/validation.rs`)

- **Added `is_ipa_file()` function**: Detects .ipa files by checking for the presence of a `Payload/` directory containing a `.app` bundle within the ZIP structure
- **Updated module exports**: Added `is_ipa_file` to the public exports in `src/utils/mobile_app/mod.rs`

### 2. IPA to XCArchive Conversion (`src/commands/mobile_app/upload.rs`)

- **Added `ipa_to_xcarchive()` function**: Converts .ipa files to xcarchive structure by:
  - Extracting the .app bundle from the `Payload/` directory
  - Creating the required xcarchive directory structure: `archive.xcarchive/Products/Applications/`
  - Copying the .app bundle to the appropriate location
  - Extracting app metadata (bundle ID, version, etc.) from the app's Info.plist
  - Generating a proper xcarchive Info.plist with required metadata
  - Creating a zip file of the complete xcarchive structure

- **Updated `normalize_file()` function**: Now detects .ipa files and automatically converts them using the new conversion function

- **Updated `validate_is_mobile_app()` function**: Added validation for .ipa files alongside existing formats

### 3. User Interface Updates

- **Command help text**: Updated to include "IPA" in the list of supported file formats
- **Error messages**: Updated to mention .ipa files in validation error messages
- **Test documentation**: Updated help test case to reflect new .ipa support

### 4. Testing

- **Added test fixture**: Created `invalid.ipa` test fixture for testing invalid .ipa file handling
- **Added test case**: `command_mobile_app_upload_invalid_ipa()` test to verify proper error handling for invalid .ipa files

## Technical Details

### IPA File Structure

An .ipa file is a ZIP archive containing:
- `Payload/` directory with a `.app` bundle
- `iTunesArtwork` (app icon)
- `iTunesMetadata.plist` (metadata)
- Additional metadata files

### XCArchive Structure Created

The conversion creates this structure:
```
archive.xcarchive/
├── Info.plist                    # Generated xcarchive metadata
└── Products/
    └── Applications/
        └── [AppName].app/         # Extracted from IPA Payload/
            ├── Info.plist         # Original app Info.plist
            ├── [executable]       # App binary
            └── [other app files]  # All other app bundle contents
```

### Generated XCArchive Info.plist

The conversion generates a standard xcarchive Info.plist containing:
- `ApplicationProperties` with app path, architectures, bundle ID, versions
- `ArchiveVersion` set to 1
- `CreationDate` set to current timestamp
- `Name` and `SchemeName` derived from app name

## Dependencies Used

The implementation leverages existing dependencies:
- `zip` crate for archive handling
- `plist` crate for parsing and generating plist files
- `chrono` crate for timestamp generation

## Backward Compatibility

This change is fully backward compatible:
- All existing functionality for .apk, .aab, and .xcarchive files remains unchanged
- New .ipa support is additive and doesn't affect existing workflows
- Error messages are enhanced but maintain the same structure

## Usage

Users can now upload .ipa files directly:

```bash
sentry-cli mobile-app upload path/to/MyApp.ipa
```

The CLI will automatically:
1. Detect the .ipa format
2. Convert it to xcarchive structure
3. Upload using the existing xcarchive upload mechanism

This provides a seamless experience for iOS developers who have .ipa files but need to upload to Sentry via the mobile-app command.