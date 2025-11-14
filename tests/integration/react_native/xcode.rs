use crate::integration::TestManager;

#[test]
fn xcode_upload_source_maps_missing_plist() {
    TestManager::new()
        .mock_common_upload_endpoints(None, None)
        .register_trycmd_test("react_native/xcode-upload-source-maps-invalid-plist.trycmd")
        .with_default_token();
}

#[test]
fn xcode_upload_source_maps_release_and_dist_from_env() {
    TestManager::new()
        .mock_common_upload_endpoints(None, Some(vec!["60f215dae7d29497357013d08c35e93716b6a46c"]))
        .register_trycmd_test(
            "react_native/xcode-upload-source-maps-release_and_dist_from_env.trycmd",
        )
        .with_default_token()
        .assert_mock_endpoints();
}
