use mockito::Mock;

#[cfg(target_os = "macos")]
use crate::integration::register_test;
#[cfg(target_os = "macos")]
use crate::integration::{mock_common_upload_endpoints, ChunkOptions, ServerBehavior};

#[test]
#[cfg(target_os = "macos")]
fn xcode_upload_source_maps_missing_plist() {
    let _upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Modern, ChunkOptions::default());
    register_test("react_native/xcode-upload-source-maps-invalid-plist.trycmd");
}

#[test]
#[cfg(target_os = "macos")]
fn xcode_upload_source_maps_release_and_dist_from_env() {
    let upload_endpoints =
        mock_common_upload_endpoints(ServerBehavior::Modern, ChunkOptions::default());
    register_test("react_native/xcode-upload-source-maps-release_and_dist_from_env.trycmd");
    assert_endpoints(&upload_endpoints);
}

pub fn assert_endpoints(mocks: &[Mock]) {
    for mock in mocks {
        mock.assert();
    }
}
