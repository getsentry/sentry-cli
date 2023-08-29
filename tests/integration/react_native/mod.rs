use crate::integration::register_test;

#[test]
fn xcode_wrap_call_minimum() {
    #[cfg(target_os = "macos")]
    register_test("react_native/xcode-wrap-call-minimum.trycmd");
    assert_empty_sourcemap_report();
    clean_up();
}

#[test]
fn xcode_wrap_call_bundle() {
    #[cfg(target_os = "macos")]
    register_test("react_native/xcode-wrap-call-bundle.trycmd");
    assert_full_sourcemap_report();
    clean_up();
}

#[test]
fn xcode_wrap_call_custom_bundle() {
    #[cfg(target_os = "macos")]
    register_test("react_native/xcode-wrap-call-custom-bundle.trycmd");
    assert_full_sourcemap_report();
    clean_up();
}

#[test]
fn xcode_wrap_call_expo_export() {
    #[cfg(target_os = "macos")]
    register_test("react_native/xcode-wrap-call-expo-export.trycmd");
    assert_full_sourcemap_report();
    clean_up();
}

fn clean_up() {
    std::fs::remove_file("rn-sourcemap-report.json").unwrap();
}

fn assert_full_sourcemap_report() {
    let actual_code =
    std::fs::read_to_string("rn-sourcemap-report.json").unwrap();
    let expected_code =
      std::fs::read_to_string("tests/integration/_fixtures/react_native/full-sourcemap-report.json.expected")
          .unwrap();

    assert_eq!(actual_code, expected_code);
}

fn assert_empty_sourcemap_report() {
    let actual_code =
    std::fs::read_to_string("rn-sourcemap-report.json").unwrap();
    let expected_code =
      std::fs::read_to_string("tests/integration/_fixtures/react_native/empty-sourcemap-report.json.expected")
          .unwrap();

    assert_eq!(actual_code, expected_code);
}