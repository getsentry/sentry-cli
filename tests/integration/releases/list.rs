use crate::integration::{mock_endpoint, register_test, EndpointOptions};

#[test]
fn displays_releases() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/wat-org/wat-project/releases/", 200)
            .with_response_file("releases/get-releases.json"),
    );
    register_test("releases/releases-list.trycmd");
}

#[test]
fn displays_releases_with_projects() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/wat-org/wat-project/releases/", 200)
            .with_response_file("releases/get-releases.json"),
    );
    register_test("releases/releases-list-with-projects.trycmd");
}

#[test]
fn doesnt_fail_with_empty_response() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/wat-org/wat-project/releases/", 200)
            .with_response_body("[]"),
    );
    register_test("releases/releases-list-empty.trycmd");
}

#[test]
fn can_override_org() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/whynot/wat-project/releases/", 200)
            .with_response_file("releases/get-releases.json"),
    );
    register_test("releases/releases-list-override-org.trycmd");
}

#[test]
fn displays_releases_in_raw_mode() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/wat-org/wat-project/releases/", 200)
            .with_response_file("releases/get-releases.json"),
    );
    register_test("releases/releases-list-raw.trycmd");
}

#[test]
fn displays_releases_in_raw_mode_with_delimiter() {
    let _server = mock_endpoint(
        EndpointOptions::new("GET", "/api/0/projects/wat-org/wat-project/releases/", 200)
            .with_response_file("releases/get-releases.json"),
    );
    register_test("releases/releases-list-raw-delimiter.trycmd");
}
