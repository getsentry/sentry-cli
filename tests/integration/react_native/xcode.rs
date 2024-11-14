use crate::integration::{ServerBehavior, TestManager};

#[test]
fn xcode_upload_source_maps_missing_plist() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test("react_native/xcode-upload-source-maps-invalid-plist.trycmd")
        .with_default_token();
}

#[test]
fn xcode_upload_source_maps_release_and_dist_from_env() {
    TestManager::new()
        .mock_common_upload_endpoints(ServerBehavior::Modern, Default::default())
        .register_trycmd_test(
            "react_native/xcode-upload-source-maps-release_and_dist_from_env.trycmd",
        )
        .with_default_token()
        .assert_mock_endpoints();
}
