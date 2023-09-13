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
    assert_packager_sourcemap_report("rn-sourcemap-report-bundle.json");
    clean_up("rn-sourcemap-report-bundle.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_custom_bundle() {
    register_test("react_native/xcode-wrap-call-custom-bundle.trycmd");
    assert_packager_sourcemap_report("rn-sourcemap-report-custom-bundle.json");
    clean_up("rn-sourcemap-report-custom-bundle.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_expo_export() {
    register_test("react_native/xcode-wrap-call-expo-export.trycmd");
    assert_packager_sourcemap_report("rn-sourcemap-report-expo-export.json");
    clean_up("rn-sourcemap-report-expo-export.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_hermesc() {
    register_test("react_native/xcode-wrap-call-hermesc.trycmd");
    assert_sourcemap_report(
        "hermesc-sourcemap-report.json.expected",
        "rn-sourcemap-report-hermesc.json",
    );
    clean_up("rn-sourcemap-report-hermesc.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_compose_source_maps() {
    std::fs::copy("tests/integration/_fixtures/react_native/compose-source-maps-sourcemap-report.json.before.test","rn-sourcemap-report-compose-source-maps.json").unwrap();
    register_test("react_native/xcode-wrap-call-compose-source-maps.trycmd");
    assert_sourcemap_report(
        "compose-source-maps-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_compose_source_maps_no_debug_id_copy() {
    std::fs::copy("tests/integration/_fixtures/react_native/compose-source-maps-sourcemap-report.json.before.test","rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json").unwrap();
    register_test("react_native/xcode-wrap-call-compose-source-maps-no-debug-id-copy.trycmd");
    assert_sourcemap_report(
        "compose-source-maps-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_wrap_call_compose_source_maps_custom() {
    register_test("react_native/xcode-wrap-call-compose-source-maps-custom.trycmd");
    assert_sourcemap_report(
        "compose-source-maps-custom-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps-custom.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps-custom.json");
}

#[cfg(target_os = "macos")]
fn clean_up(path: &str) {
    std::fs::remove_file(path).unwrap();
}

#[cfg(target_os = "macos")]
fn assert_packager_sourcemap_report(actual: &str) {
    assert_sourcemap_report("packager-sourcemap-report.json.expected", actual);
}

#[cfg(target_os = "macos")]
fn assert_empty_sourcemap_report(actual: &str) {
    assert_sourcemap_report("empty-sourcemap-report.json.expected", actual);
}

#[cfg(target_os = "macos")]
fn assert_sourcemap_report(expected: &str, actual: &str) {
    let actual_code = std::fs::read_to_string(actual).unwrap();
    let expected_code =
        std::fs::read_to_string("tests/integration/_fixtures/react_native/".to_owned() + expected)
            .unwrap();

    assert_eq!(actual_code, expected_code);
}
