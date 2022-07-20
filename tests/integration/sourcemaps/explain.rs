use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn command_sourcemaps_explain_help() {
    register_test("sourcemaps/sourcemaps-explain-help.trycmd");
}

#[test]
fn command_sourcemaps_explain() {
    register_test("sourcemaps/sourcemaps-explain.trycmd");
}

#[test]
fn command_sourcemaps_explain_missing_event() {
    let _event = mock_endpoint(EndpointOptions::new(
        "GET",
        "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
        404,
    ));
    register_test("sourcemaps/sourcemaps-explain-missing-event.trycmd");
}

#[test]
fn command_sourcemaps_explain_missing_release() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-release.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-missing-release.trycmd");
}

#[test]
fn command_sourcemaps_explain_missing_exception() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-exception.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-missing-exception.trycmd");
}

#[test]
fn command_sourcemaps_explain_missing_stacktrace() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-stacktrace.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-missing-stacktrace.trycmd");
}

#[test]
fn command_sourcemaps_explain_frame_no_inapp() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-frame-no-inapp.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-frame-no-inapp.trycmd");
}

#[test]
fn command_sourcemaps_explain_frame_no_abspath() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-frame-no-abspath.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-frame-no-abspath.trycmd");
}

#[test]
fn command_sourcemaps_explain_frame_no_extension() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-frame-no-extension.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-frame-no-extension.trycmd");
}

#[test]
fn command_sourcemaps_explain_frame_malformed_abspath() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-frame-malformed-abspath.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-frame-malformed-abspath.trycmd");
}

#[test]
fn command_sourcemaps_explain_already_mapped() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-already-mapped.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-already-mapped.trycmd");
}

#[test]
fn command_sourcemaps_explain_no_artifacts() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-empty.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-no-artifacts.trycmd");
}

#[test]
fn command_sourcemaps_explain_no_matching_artifact() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-match.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-no-matching-artifact.trycmd");
}

#[test]
fn command_sourcemaps_explain_partial_matching_artifact() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-partial-match.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-partial-matching-artifact.trycmd");
}

#[test]
fn command_sourcemaps_explain_artifact_dist_mismatch() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-artifact-dist-mismatch.trycmd");
}

#[test]
fn command_sourcemaps_explain_artifact_no_dist() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-dist.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-artifact-no-dist.trycmd");
}

#[test]
fn command_sourcemaps_explain_event_no_dist() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-dist.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-event-no-dist.trycmd");
}

#[test]
fn command_sourcemaps_explain_detect_from_sourcemap_header() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-dist.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
    );
    let _file_metadata = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            200,
        )
        .with_response_file("sourcemaps/get-file-metadata-sourcemap-header.json"),
    );

    register_test("sourcemaps/sourcemaps-explain-detect-from-sourcemap-header.trycmd");
}

#[test]
fn command_sourcemaps_explain_detect_from_xsourcemap_header() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-dist.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
    );
    let _file_metadata = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            200,
        )
        .with_response_file("sourcemaps/get-file-metadata-xsourcemap-header.json"),
    );

    register_test("sourcemaps/sourcemaps-explain-detect-from-xsourcemap-header.trycmd");
}

#[test]
fn command_sourcemaps_explain_detect_from_file_content() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-dist.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-sourcemap.json"),
    );
    let _file_metadata = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            200,
        )
        .with_response_file("sourcemaps/get-file-metadata-no-headers.json"),
    );
    let _file = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/?download=1",
            200,
        )
        .with_response_file("sourcemaps/get-file.js"),
    );

    register_test("sourcemaps/sourcemaps-explain-detect-from-file-content.trycmd");
}

#[test]
fn command_sourcemaps_explain_print_sourcemap() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-missing-dist.json"),
    );
    let _artifacts = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/?cursor=",
            200,
        )
        .with_response_file("sourcemaps/get-artifacts-no-dist.json"),
    );
    let _file_metadata = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495645/",
            200,
        )
        .with_response_file("sourcemaps/get-file-metadata-sourcemap-header.json"),
    );
    let _file = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/releases/ytho-test/files/6796495646/?download=1",
            200,
        )
        .with_response_file("sourcemaps/get-file-sourcemap.js.map"),
    );

    register_test("sourcemaps/sourcemaps-explain-print-sourcemap.trycmd");
}

#[test]
fn command_sourcemaps_explain_select_frame() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-select-frame.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-select-frame.trycmd");
}

#[test]
fn command_sourcemaps_explain_select_frame_out_of_range() {
    let _event = mock_endpoint(
        EndpointOptions::new(
            "GET",
            "/api/0/projects/wat-org/wat-project/events/43a57a55cd5a4207ac520c03e1dee1b4/json/",
            200,
        )
        .with_response_file("sourcemaps/get-event-select-frame.json"),
    );
    register_test("sourcemaps/sourcemaps-explain-select-frame-out-of-range.trycmd");
}
