#[cfg(target_os = "macos")]
use crate::integration::register_test;

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_minimum() {
    register_test("react_native/xcode-wrap-call-minimum.trycmd");
    assert_empty_sourcemap_report("rn-sourcemap-report-minimum.json");
    clean_up("rn-sourcemap-report-minimum.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_bundle() {
    register_test("react_native/xcode-wrap-call-bundle.trycmd");
    assert_full_sourcemap_report("rn-sourcemap-report-bundle.json");
    clean_up("rn-sourcemap-report-bundle.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_custom_bundle() {
    register_test("react_native/xcode-wrap-call-custom-bundle.trycmd");
    assert_full_sourcemap_report("rn-sourcemap-report-custom-bundle.json");
    clean_up("rn-sourcemap-report-custom-bundle.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_expo_export() {
    register_test("react_native/xcode-wrap-call-expo-export.trycmd");
    assert_full_sourcemap_report("rn-sourcemap-report-expo-export.json");
    clean_up("rn-sourcemap-report-expo-export.json");
}

fn clean_up(path: &str) {
    std::fs::remove_file(path).unwrap();
}

fn assert_full_sourcemap_report(actual: &str) {
    let actual_code = std::fs::read_to_string(actual).unwrap();
    let expected_code = std::fs::read_to_string(
        "tests/integration/_fixtures/react_native/full-sourcemap-report.json.expected",
    )
    .unwrap();

    assert_eq!(actual_code, expected_code);
}

fn assert_empty_sourcemap_report(actual: &str) {
    let actual_code = std::fs::read_to_string(actual).unwrap();
    let expected_code = std::fs::read_to_string(
        "tests/integration/_fixtures/react_native/empty-sourcemap-report.json.expected",
    )
    .unwrap();

    assert_eq!(actual_code, expected_code);
}
