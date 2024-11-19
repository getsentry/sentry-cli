use crate::integration::{MockEndpointBuilder, TestManager};

#[test]
fn command_sourcemaps_explain_help() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-explain-help.trycmd");
}

#[test]
fn command_sourcemaps_explain() {
    TestManager::new().register_trycmd_test("sourcemaps/sourcemaps-explain.trycmd");
}

#[test]
fn command_sourcemaps_explain_missing_event() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_status(404),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-missing-event.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_missing_release() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-release.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-missing-release.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_missing_exception() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-exception.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-missing-exception.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_missing_stacktrace() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-stacktrace.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-missing-stacktrace.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_frame_no_inapp() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-frame-no-inapp.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-frame-no-inapp.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_frame_no_abspath() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-frame-no-abspath.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-frame-no-abspath.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_frame_no_extension() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-frame-no-extension.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-frame-no-extension.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_frame_malformed_abspath() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-frame-malformed-abspath.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-frame-malformed-abspath.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_already_mapped() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-already-mapped.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-already-mapped.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_no_artifacts() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-empty.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-no-artifacts.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_no_matching_artifact() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-match.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-no-matching-artifact.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_partial_matching_artifact() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-partial-match.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-partial-matching-artifact.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_artifact_dist_mismatch() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-artifact-dist-mismatch.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_artifact_no_dist() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-dist.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-artifact-no-dist.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_event_no_dist() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-event-no-dist.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_detect_from_sourcemap_header() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            )
            .with_response_file("sourcemaps/get-file-metadata-sourcemap-header.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-detect-from-sourcemap-header.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_detect_from_xsourcemap_header() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            )
            .with_response_file("sourcemaps/get-file-metadata-xsourcemap-header.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-detect-from-xsourcemap-header.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_detect_from_file_content() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            )
            .with_response_file("sourcemaps/get-file-metadata-no-headers.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/?download=1",
            )
            .with_response_file("sourcemaps/get-file.js"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-detect-from-file-content.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_print_sourcemap() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-missing-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            )
            .with_response_file("sourcemaps/get-artifacts-no-dist.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            )
            .with_response_file("sourcemaps/get-file-metadata-sourcemap-header.json"),
        )
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495646/?download=1",
            )
            .with_response_file("sourcemaps/get-file-sourcemap.js.map"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-print-sourcemap.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_select_frame() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-select-frame.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-select-frame.trycmd")
        .with_default_token();
}

#[test]
fn command_sourcemaps_explain_select_frame_out_of_range() {
    TestManager::new()
        .mock_endpoint(
            MockEndpointBuilder::new(
                "GET",
                "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            )
            .with_response_file("sourcemaps/get-event-select-frame.json"),
        )
        .register_trycmd_test("sourcemaps/sourcemaps-explain-select-frame-out-of-range.trycmd")
        .with_default_token();
}
