### Artifact assembly

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/artifactbundle/assemble/` | `POST` | No | No | `ingest` | `sentry-api-0-organization-artifactbundle-assemble` | `sentry.api.endpoints.organization_artifactbundle_assemble.OrganizationArtifactBundleAssembleEndpoint` | `api::AuthenticatedApi::assemble_artifact_bundle` |  |
| `/organizations/{org}/releases/{release}/assemble/` | `POST` | No | No | `unowned` | `sentry-api-0-organization-release-assemble` | `sentry.releases.endpoints.organization_release_assemble.OrganizationReleaseAssembleEndpoint` | `api::AuthenticatedApi::assemble_release_artifacts` |  |
| `/projects/{org}/{project}/files/difs/assemble/` | `POST` | No | No | `ingest` | `sentry-api-0-assemble-dif-files` | `sentry.api.endpoints.debug_files.DifAssembleEndpoint` | `api::AuthenticatedApi::assemble_difs` |  |
| `/projects/{org}/{project}/files/preprodartifacts/assemble/` | `POST` | No | No | `emerge-tools` | `sentry-api-0-assemble-preprod-artifact-files` | `sentry.preprod.api.endpoints.organization_preprod_artifact_assemble.ProjectPreprodArtifactAssembleEndpoint` | `api::AuthenticatedApi::assemble_build` |  |



### Authentication and user info

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/` | `GET` | No | No | `unowned` | `sentry-api-index` | `sentry.api.endpoints.index.IndexEndpoint` | `api::AuthenticatedApi::get_auth_info` | Returns auth/user info |
| `/users/me/regions/` | `GET` | No | No | `hybrid-cloud` | `sentry-api-0-user-regions` | `sentry.users.api.endpoints.user_regions.UserRegionsEndpoint` | `api::AuthenticatedApi::list_available_regions` | May be 404 on self-hosted; returns [] then |



### Debug information files (DIFs) and chunk upload

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/chunk-upload/` | `GET` | No | No | `ingest` | `sentry-api-0-chunk-upload` | `sentry.api.endpoints.chunk.ChunkUploadEndpoint` | `api::AuthenticatedApi::get_chunk_upload_options` | Returns server-provided absolute upload URL |
| `/projects/{org}/{project}/files/dsyms/unknown/` | `GET` | No | No | `ingest` | `sentry-api-0-unknown-dsym-files` | `sentry.api.endpoints.debug_files.UnknownDebugFilesEndpoint` | `api::AuthenticatedApi::find_missing_dif_checksums` |  |



### Deploys

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/releases/{version}/deploys/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-release-deploys` | `sentry.releases.endpoints.release_deploys.ReleaseDeploysEndpoint` | `api::AuthenticatedApi::list_deploys` |  |
| `/organizations/{org}/releases/{version}/deploys/` | `POST` | No | No | `unowned` | `sentry-api-0-organization-release-deploys` | `sentry.releases.endpoints.release_deploys.ReleaseDeploysEndpoint` | `api::AuthenticatedApi::create_deploy` |  |



### Events and logs

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/events/` | `GET` | No | No | `visibility` | `sentry-api-0-organization-events` | `sentry.api.endpoints.organization_events.OrganizationEventsEndpoint` | `api::AuthenticatedApi::fetch_organization_events` | Dataset-backed (e.g., dataset=logs); multiple field params; optional cursor, project |
| `/organizations/{org}/events/{event_id}/json/` | `GET` | No | No | `unowned` | `sentry-api-catchall` | `sentry.api.endpoints.catchall.CatchallEndpoint` | `api::AuthenticatedApi::get_event` |  |
| `/projects/{org}/{project}/events/` | `GET` | No | No | `issue-workflow` | `sentry-api-0-project-events` | `sentry.issues.endpoints.project_events.ProjectEventsEndpoint` | `api::AuthenticatedApi::list_organization_project_events` |  |
| `/projects/{org}/{project}/events/{event_id}/json/` | `GET` | No | No | `issue-workflow` | `sentry-api-0-event-json` | `sentry.issues.endpoints.project_event_details.EventJsonEndpoint` | `api::AuthenticatedApi::get_event` |  |
| `/projects/{org}/{project}/issues/` | `GET` | No | No | `issue-workflow` | `sentry-api-0-project-group-index` | `sentry.issues.endpoints.project_group_index.ProjectGroupIndexEndpoint` | `api::AuthenticatedApi::list_organization_project_issues` |  |
| `/projects/{org}/{project}/issues/` | `PUT` | No | No | `issue-workflow` | `sentry-api-0-project-group-index` | `sentry.issues.endpoints.project_group_index.ProjectGroupIndexEndpoint` | `api::AuthenticatedApi::bulk_update_issue` | Query string built from IssueFilter |



### Monitors

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/monitors/` | `GET` | No | No | `crons` | `sentry-api-0-organization-monitor-index` | `sentry.monitors.endpoints.organization_monitor_index.OrganizationMonitorIndexEndpoint` | `api::AuthenticatedApi::list_organization_monitors` |  |



### Organizations

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/` | `GET` | Yes | No | `unowned` | `sentry-api-0-organizations` | `sentry.core.endpoints.organization_index.OrganizationIndexEndpoint` | `api::AuthenticatedApi::list_organizations` |  |
| `/organizations/{org}/region/` | `GET` | No | No | `hybrid-cloud` | `sentry-api-0-organization-region` | `sentry.core.endpoints.organization_region.OrganizationRegionEndpoint` | `api::AuthenticatedApi::get_region_url` |  |



### Proguard

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/projects/{org}/{project}/files/proguard-artifact-releases` | `POST` | No | No | `ingest` | `sentry-api-0-proguard-artifact-releases` | `sentry.api.endpoints.debug_files.ProguardArtifactReleasesEndpoint` | `api::AuthenticatedApi::associate_proguard_mappings` |  |



### Projects

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/projects/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-projects` | `sentry.core.endpoints.organization_projects.OrganizationProjectsEndpoint` | `api::AuthenticatedApi::list_organization_projects` |  |



### Region-specific uploads (wrapper)

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/projects/{org}/{project}/files/dsyms/` | `POST` | Yes | No | `ingest` | `sentry-api-0-dsym-files` | `sentry.api.endpoints.debug_files.DebugFilesEndpoint` | `api::RegionSpecificApi::upload_dif_archive` |  |



### Release files (artifacts)

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/files/source-maps/` | `DELETE` | No | No | `unowned` | `sentry-api-catchall` | `sentry.api.endpoints.catchall.CatchallEndpoint` | `api::AuthenticatedApi::delete_release_files` |  |
| `/organizations/{org}/releases/{release}/files/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-release-files` | `sentry.releases.endpoints.organization_release_files.OrganizationReleaseFilesEndpoint` | `api::AuthenticatedApi::list_release_files_by_checksum` |  |
| `/organizations/{org}/releases/{release}/files/` | `POST` | Yes | No | `unowned` | `sentry-api-0-organization-release-files` | `sentry.releases.endpoints.organization_release_files.OrganizationReleaseFilesEndpoint` | `api::RegionSpecificApi::upload_release_file` | multipart |
| `/organizations/{org}/releases/{version}/files/{file_id}/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-release-file-details` | `sentry.releases.endpoints.organization_release_file_details.OrganizationReleaseFileDetailsEndpoint` | `api::AuthenticatedApi::get_release_file, api::AuthenticatedApi::get_release_file_metadata` |  |
| `/organizations/{org}/releases/{version}/files/{file_id}/` | `DELETE` | No | No | `unowned` | `sentry-api-0-organization-release-file-details` | `sentry.releases.endpoints.organization_release_file_details.OrganizationReleaseFileDetailsEndpoint` | `api::AuthenticatedApi::delete_release_file` |  |
| `/projects/{org}/{project}/files/source-maps/` | `DELETE` | No | No | `ingest` | `sentry-api-0-source-maps` | `sentry.api.endpoints.debug_files.SourceMapsEndpoint` | `api::AuthenticatedApi::delete_release_files` |  |
| `/projects/{org}/{project}/releases/{release}/files/` | `GET` | No | No | `unowned` | `sentry-api-0-project-release-files` | `sentry.releases.endpoints.project_release_files.ProjectReleaseFilesEndpoint` | `api::AuthenticatedApi::list_release_files_by_checksum` |  |
| `/projects/{org}/{project}/releases/{release}/files/` | `POST` | Yes | No | `unowned` | `sentry-api-0-project-release-files` | `sentry.releases.endpoints.project_release_files.ProjectReleaseFilesEndpoint` | `api::RegionSpecificApi::upload_release_file` | multipart |
| `/projects/{org}/{project}/releases/{version}/files/{file_id}/` | `GET` | No | No | `unowned` | `sentry-api-0-project-release-file-details` | `sentry.releases.endpoints.project_release_file_details.ProjectReleaseFileDetailsEndpoint` | `api::AuthenticatedApi::get_release_file, api::AuthenticatedApi::get_release_file_metadata` |  |
| `/projects/{org}/{project}/releases/{version}/files/{file_id}/` | `DELETE` | No | No | `unowned` | `sentry-api-0-project-release-file-details` | `sentry.releases.endpoints.project_release_file_details.ProjectReleaseFileDetailsEndpoint` | `api::AuthenticatedApi::delete_release_file` |  |



### Releases

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/releases/` | `POST` | No | No | `unowned` | `sentry-api-0-organization-releases` | `sentry.api.endpoints.organization_releases.OrganizationReleasesEndpoint` | `api::AuthenticatedApi::new_release, api::AuthenticatedApi::update_release` |  |
| `/organizations/{org}/releases/{version}/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-release-details` | `sentry.releases.endpoints.organization_release_details.OrganizationReleaseDetailsEndpoint` | `api::AuthenticatedApi::get_release` |  |
| `/organizations/{org}/releases/{version}/` | `POST` | No | No | `unowned` | `sentry-api-0-organization-release-details` | `sentry.releases.endpoints.organization_release_details.OrganizationReleaseDetailsEndpoint` | `api::AuthenticatedApi::update_release` | Creates a replacement when updating version |
| `/organizations/{org}/releases/{version}/` | `PUT` | No | No | `unowned` | `sentry-api-0-organization-release-details` | `sentry.releases.endpoints.organization_release_details.OrganizationReleaseDetailsEndpoint` | `api::AuthenticatedApi::update_release, set_release_refs` |  |
| `/organizations/{org}/releases/{version}/` | `DELETE` | No | No | `unowned` | `sentry-api-0-organization-release-details` | `sentry.releases.endpoints.organization_release_details.OrganizationReleaseDetailsEndpoint` | `api::AuthenticatedApi::delete_release` |  |
| `/organizations/{org}/releases/{version}/commits/` | `GET` | No | No | `unowned` | `sentry-api-0-organization-release-commits` | `sentry.releases.endpoints.organization_release_commits.OrganizationReleaseCommitsEndpoint` | `api::AuthenticatedApi::get_release_commits` |  |
| `/organizations/{org}/releases/{version}/previous-with-commits/` | `GET` | No | No | `issue-workflow` | `sentry-api-0-organization-release-previous-with-commits` | `sentry.issues.endpoints.organization_release_previous_commits.OrganizationReleasePreviousCommitsEndpoint` | `api::AuthenticatedApi::get_previous_release_with_commits` |  |
| `/projects/{org}/{project}/releases/` | `GET` | No | No | `unowned` | `sentry-api-0-project-releases` | `sentry.releases.endpoints.project_releases.ProjectReleasesEndpoint` | `api::AuthenticatedApi::list_releases` |  |
| `/projects/{org}/{project}/releases/` | `POST` | No | No | `unowned` | `sentry-api-0-project-releases` | `sentry.releases.endpoints.project_releases.ProjectReleasesEndpoint` | `api::AuthenticatedApi::new_release` | Legacy single-project endpoint |
| `/projects/{org}/{project}/releases/{version}/` | `GET` | No | No | `unowned` | `sentry-api-0-project-release-details` | `sentry.releases.endpoints.project_release_details.ProjectReleaseDetailsEndpoint` | `api::AuthenticatedApi::get_release` |  |
| `/projects/{org}/{project}/releases/{version}/` | `PUT` | No | No | `unowned` | `sentry-api-0-project-release-details` | `sentry.releases.endpoints.project_release_details.ProjectReleaseDetailsEndpoint` | `api::AuthenticatedApi::update_release` |  |
| `/projects/{org}/{project}/releases/{version}/` | `DELETE` | No | No | `unowned` | `sentry-api-0-project-release-details` | `sentry.releases.endpoints.project_release_details.ProjectReleaseDetailsEndpoint` | `api::AuthenticatedApi::delete_release` |  |
| `/projects/{org}/{project}/releases/{version}/commits/` | `GET` | No | No | `unowned` | `sentry-api-0-project-release-commits` | `sentry.releases.endpoints.project_release_commits.ProjectReleaseCommitsEndpoint` | `api::AuthenticatedApi::get_release_commits` |  |



### Repositories

| Path | Method | Region-aware | Absolute | Owner | Endpoint Name | Class (in backend) | Locations used (in CLI) | Notes |
| ---- | ------ | ------------ | -------- | ----- | ------------- | ------------------- | ----------------------- | ----- |
| `/organizations/{org}/repos/` | `GET` | No | No | `product-owners-settings-integrations` | `sentry-api-0-organization-repositories` | `sentry.integrations.api.endpoints.organization_repositories.OrganizationRepositoriesEndpoint` | `api::AuthenticatedApi::list_organization_repos` |  |
