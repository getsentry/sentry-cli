use std::fs;
use std::fs::remove_dir_all;
use std::path::Path;

use crate::integration::{copy_recursively, test_utils::AssertCommand, TestManager};

#[test]
fn command_sourcemaps_inject_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-help.trycmd");
}

#[test]
fn command_sourcemaps_inject_output() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively("tests/integration/_fixtures/inject/", testcase_cwd_path).unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject",
    );
}

#[test]
fn command_sourcemaps_inject_output_nomappings() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-nomappings.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_nomappings/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-nomappings.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-nomappings",
    );
}

#[test]
fn command_sourcemaps_inject_output_nofiles() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-nofiles.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    fs::create_dir_all(std::path::Path::new(testcase_cwd_path).join("nonexisting")).unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-nofiles.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-nofiles",
    );
}

#[test]
fn command_sourcemaps_inject_output_embedded() {
    let testcase_cwd_path =
        std::path::Path::new("tests/integration/_cases/sourcemaps/sourcemaps-inject-embedded.in/");
    if testcase_cwd_path.exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    fs::create_dir_all(testcase_cwd_path).unwrap();
    fs::copy(
        "tests/integration/_fixtures/inject/server/dummy_embedded.js",
        testcase_cwd_path.join("dummy_embedded.js"),
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-embedded.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-embedded",
    );
}

#[test]
fn command_sourcemaps_inject_output_split() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-split.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_split/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-split.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-split",
    );
}

#[test]
fn command_sourcemaps_inject_output_split_ambiguous() {
    let testcase_cwd_path =
        "tests/integration/_cases/sourcemaps/sourcemaps-inject-split-ambiguous.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_split_ambiguous/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-split-ambiguous.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-split-ambiguous",
    );
}

#[test]
fn command_sourcemaps_inject_bundlers() {
    let testcase_cwd_path = "tests/integration/_cases/sourcemaps/sourcemaps-inject-bundlers.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_bundlers/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-bundlers.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-bundlers",
    );
}

#[test]
fn command_sourcemaps_inject_not_compiled() {
    let testcase_cwd_path =
        "tests/integration/_cases/sourcemaps/sourcemaps-inject-not-compiled.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }

    copy_recursively(
        "tests/integration/_fixtures/inject-not-compiled",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-not-compiled.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-not-compiled",
    );
}

#[test]
fn command_sourcemaps_inject_complex_extension() {
    TestManager::new()
        .register_trycmd_test("sourcemaps/sourcemaps-inject-complex-extension.trycmd");
}

#[test]
fn command_sourcemaps_inject_indexed() {
    const FIXTURE_PATH: &str = "tests/integration/_fixtures/inject_indexed/";
    const TESTCASE_PATH: &str = "tests/integration/_cases/sourcemaps/sourcemaps-inject-indexed.in/";
    const EXPECTED_OUTPUT_PATH: &str =
        "tests/integration/_expected_outputs/sourcemaps/inject_indexed/";

    // Setup the working directory
    if std::path::Path::new(TESTCASE_PATH).exists() {
        remove_dir_all(TESTCASE_PATH).expect("Failed to remove working directory");
    }
    copy_recursively(FIXTURE_PATH, TESTCASE_PATH).expect("Failed to copy inject_indexed");

    // Run the inject command against the working directory
    TestManager::new()
        .assert_cmd(vec!["sourcemaps", "inject", TESTCASE_PATH])
        .run_and_assert(AssertCommand::Success);

    // The rest of the test just compares the actual output with the expected output
    assert_directories_equal(TESTCASE_PATH, EXPECTED_OUTPUT_PATH);
}

#[test]
fn command_sourcemaps_inject_double_association() {
    let testcase_cwd_path =
        "tests/integration/_cases/sourcemaps/sourcemaps-inject-double-association.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/inject_double_association/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new()
        .register_trycmd_test("sourcemaps/sourcemaps-inject-double-association.trycmd");

    assert_directories_equal(
        testcase_cwd_path,
        "tests/integration/_expected_outputs/sourcemaps/sourcemaps-inject-double-association",
    );
}

#[test]
fn command_sourcemaps_inject_ignore_relative() {
    let testcase_cwd_path =
        "tests/integration/_cases/sourcemaps/sourcemaps-inject-ignore-relative.in/";
    if std::path::Path::new(testcase_cwd_path).exists() {
        remove_dir_all(testcase_cwd_path).unwrap();
    }
    copy_recursively(
        "tests/integration/_fixtures/ignore_test/",
        testcase_cwd_path,
    )
    .unwrap();

    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-inject-ignore-relative.trycmd");
}

/// Recursively assert that the contents of two directories are equal.
///
/// We only support directories that contain exclusively text files.
///
/// Any .gitkeep files are ignored. We also normalize line endings to UNIX line endings
/// when running the comparison on Windows.
///
/// Panics if there is any difference between the two directories (e.g. if there are different
/// numbers of files, or if there are files with different contents).
fn assert_directories_equal(actual_path: impl AsRef<Path>, expected_path: impl AsRef<Path>) {
    let mut actual_dir: Vec<_> = fs::read_dir(&actual_path)
        .and_then(|dir| dir.collect())
        .unwrap_or_else(|_| {
            panic!(
                "error while reading actual directory: {}",
                actual_path.as_ref().display()
            )
        });
    let mut expected_dir: Vec<_> = fs::read_dir(&expected_path)
        .and_then(|dir| {
            dir.filter(|entry| {
                // Filter out any .gitkeep files.
                entry
                    .as_ref()
                    .map(|entry| entry.file_name() != ".gitkeep")
                    .unwrap_or(true)
            })
            .collect()
        })
        .unwrap_or_else(|_| {
            panic!(
                "error while reading expected directory: {}",
                expected_path.as_ref().display()
            )
        });

    actual_dir.sort_unstable_by_key(|entry| entry.file_name());
    expected_dir.sort_unstable_by_key(|entry| entry.file_name());

    assert_eq!(
        actual_dir.len(),
        expected_dir.len(),
        "the directories {} and {} have different numbers of files",
        actual_path.as_ref().display(),
        expected_path.as_ref().display()
    );
    for (actual_entry, expected_entry) in actual_dir.iter().zip(expected_dir.iter()) {
        if expected_entry
            .file_type()
            .expect("error while reading expected file type")
            .is_dir()
        {
            assert_directories_equal(actual_entry.path(), expected_entry.path());
            continue;
        }

        let actual_contents =
            std::fs::read_to_string(actual_entry.path()).expect("error while reading actual file");
        let expected_contents = std::fs::read_to_string(expected_entry.path())
            .expect("error while reading expected file");

        #[cfg(windows)]
        // The expected output is formatted with UNIX line endings.
        let actual_contents = actual_contents.replace("\r\n", "\n");

        #[cfg(windows)]
        // The expected output is formatted with UNIX line endings.
        let expected_contents = expected_contents.replace("\r\n", "\n");

        assert_eq!(
            actual_contents,
            expected_contents,
            "the contents of {} and {} differ",
            actual_entry.path().display(),
            expected_entry.path().display()
        );
    }
}
