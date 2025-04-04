use crate::integration::TestManager;

mod xcode;

#[test]
fn xcode_wrap_call_minimum() {
    TestManager::new().register_trycmd_test("react_native/xcode-wrap-call-minimum.trycmd");

    assert_empty_sourcemap_report("rn-sourcemap-report-minimum.json");
    clean_up("rn-sourcemap-report-minimum.json");
}

#[test]
fn xcode_wrap_call_bundle() {
    TestManager::new().register_trycmd_test("react_native/xcode-wrap-call-bundle.trycmd");

    assert_packager_sourcemap_report("rn-sourcemap-report-bundle.json");
    clean_up("rn-sourcemap-report-bundle.json");
}

#[test]
fn xcode_wrap_call_custom_bundle() {
    TestManager::new().register_trycmd_test("react_native/xcode-wrap-call-custom-bundle.trycmd");

    assert_packager_sourcemap_report("rn-sourcemap-report-custom-bundle.json");
    clean_up("rn-sourcemap-report-custom-bundle.json");
}

#[test]
fn xcode_wrap_call_expo_export() {
    TestManager::new().register_trycmd_test("react_native/xcode-wrap-call-expo-export.trycmd");

    assert_packager_sourcemap_report("rn-sourcemap-report-expo-export.json");
    clean_up("rn-sourcemap-report-expo-export.json");
}

#[test]
fn xcode_wrap_call_hermesc() {
    TestManager::new().register_trycmd_test("react_native/xcode-wrap-call-hermesc.trycmd");

    assert_sourcemap_report(
        "hermesc-sourcemap-report.json.expected",
        "rn-sourcemap-report-hermesc.json",
    );
    clean_up("rn-sourcemap-report-hermesc.json");
}

#[test]
fn xcode_wrap_call_compose_source_maps() {
    std::fs::copy("tests/integration/_fixtures/react_native/compose-source-maps-sourcemap-report.json.before.test","rn-sourcemap-report-compose-source-maps.json").unwrap();

    TestManager::new()
        .register_trycmd_test("react_native/xcode-wrap-call-compose-source-maps.trycmd");

    assert_sourcemap_report(
        "compose-source-maps-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps.json");
}

#[test]
fn xcode_wrap_call_compose_source_maps_no_debug_id_copy() {
    std::fs::copy("tests/integration/_fixtures/react_native/compose-source-maps-sourcemap-report.json.before.test","rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json").unwrap();

    TestManager::new().register_trycmd_test(
        "react_native/xcode-wrap-call-compose-source-maps-no-debug-id-copy.trycmd",
    );

    assert_sourcemap_report(
        "compose-source-maps-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps-no-debug-id-copy.json");
}

#[test]
fn xcode_wrap_call_compose_source_maps_custom() {
    TestManager::new()
        .register_trycmd_test("react_native/xcode-wrap-call-compose-source-maps-custom.trycmd");

    assert_sourcemap_report(
        "compose-source-maps-custom-sourcemap-report.json.expected",
        "rn-sourcemap-report-compose-source-maps-custom.json",
    );
    clean_up("rn-sourcemap-report-compose-source-maps-custom.json");
}

fn clean_up(path: &str) {
    std::fs::remove_file(path).unwrap();
}

fn assert_packager_sourcemap_report(actual: &str) {
    assert_sourcemap_report("packager-sourcemap-report.json.expected", actual);
}

fn assert_empty_sourcemap_report(actual: &str) {
    assert_sourcemap_report("empty-sourcemap-report.json.expected", actual);
}

fn assert_sourcemap_report(expected: &str, actual: &str) {
    let actual_code = std::fs::read_to_string(actual).unwrap();
    let expected_code =
        std::fs::read_to_string("tests/integration/_fixtures/react_native/".to_owned() + expected)
            .unwrap();

    assert_eq!(actual_code, expected_code);
}
