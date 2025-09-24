Unlisted relative Sentry API endpoints used by sentry-cli

- "/organizations/" (with pagination `?cursor=`)
  - Where: `src/api/mod.rs` in `AuthenticatedApi::list_organizations`
  - Note: Your list includes "/organizations/" but not explicitly the listing endpoint with pagination. Pagination query params are ignored per instructions; keeping here only to show callsite coverage.

- "/projects/{org}/{project}/releases/{version}/files/{file_id}/?download=1"
  - Where: `src/api/mod.rs` in `AuthenticatedApi::get_release_file`
  - Status: NOT in provided list (download variant)

- "/organizations/{org}/releases/{version}/files/{file_id}/?download=1"
  - Where: `src/api/mod.rs` in `AuthenticatedApi::get_release_file`
  - Status: NOT in provided list (download variant)

- "/monitors/{monitor_slug}/checkins/"
  - Where: Sent via the Envelopes API (DSN envelope endpoint). Tests mock this relative REST endpoint: `tests/integration/monitors.rs`
  - Status: NOT in provided list. Although CLI sends monitor check-ins as envelopes to the DSN endpoint, the relative check-ins endpoint appears in tests.

- "/projects/{org}/{project}/files/proguard-artifact-releases"
  - Where: `src/api/mod.rs` in `AuthenticatedApi::associate_proguard_mappings`
  - Status: In provided list

- "/projects/{org}/{project}/files/dsyms/"
  - Where: `src/api/mod.rs` in `RegionSpecificApi::upload_dif_archive`
  - Status: In provided list

- "/projects/{org}/{project}/releases/{release}/files/"
  - Where: `src/api/mod.rs` in `RegionSpecificApi::upload_release_file`
  - Status: In provided list

No other relative endpoints beyond those already listed were found in `src/` (including releases, deploys, issues, events, artifacts, chunk upload, assemble endpoints, regions, repos, monitors list, projects list). JavaScript wrapper code does not add additional relative API endpoints.

