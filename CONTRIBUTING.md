# Integration Tests

Integration tests are written using `trycmd` crate. Consult the docs in case you need to understand how it works https://docs.rs/trycmd/latest/trycmd/.

The main parts to remember are:
- `register_test` already understands that all tests will live under `tests/integration/_cases`
- use mocks for API responses
- use fixtures for uploading/processing predefined data
- write separate tests for Windows when necessary (using `#[cfg(windows)]` attribute)
- use wildcard for dynamic output (eg. timestamps or UUIDs) (explained in `trycmd` docs - eg. `[..]` or `[EXE]`) or anything that is platform specific, as tests are run on Linux, OSX and Windows.
- `Usage:` help prompt _always_ requires `[EXE]` wildcard, so make sure to not forget it

## Updating Snapshots

In order to overwrite current integration tests snapshots, use `TRYCMD=overwrite` env variable when running tests, eg.

```shell
$ TRYCMD=overwrite cargo test
```

## Working with Fixtures

To run tests with specific fixtures in isolation, utilize the fact that `trycmd` is automatically creating and using `.in` as CWD and `.out` as stdout directories respectively for every test. This allows us to use eg. `tests/integration/_cases/sourcemaps/sourcemaps-inject.in/` path as a sandbox for `tests/integration/_cases/sourcemaps/sourcemaps-inject.trycmd` test case.
You can copy/remove any files programmatically from those directories and they will be ignored from the repository.

Here's basic test that use fixtures in isolation:

```rust
fn command_sourcemaps_inject_output() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/inject/", testcase_cwd_path).unwrap();

    register_test("sourcemaps/sourcemaps-inject.trycmd");
}
```

## Working with API Mocks

If you are trying mock an API response, use `mock_endpoint` helper with `with_response_file` method called,
and place the JSON file in an appropriate directory under `tests/integration/_responses`.
Make sure to assign the mock to a variable, otherwise it won't be picked up when creating a server.

```rust
let _assemble = mock_endpoint(
    EndpointOptions::new(
        "POST",
        "/api/0/projects/wat-org/wat-project/files/difs/assemble/",
        200,
    )
    .with_response_file("debug_files/post-difs-assemble.json"),
);
```

If you don't know what APIs will be hit during your test, register the test as normal, using `register_test` helper,
and run it in isolation, eg. `cargo test command_debug_files_upload`.
This will store all original API responses under `dump/` directory (controlled via. `SENTRY_DUMP_RESPONSES` env property set
in `tests/integration/mod`), where all path separators `/` are replaced with double underscore `__`.
This way you can simply copy the JSON files to `_responses` directory and update the data as needed.
