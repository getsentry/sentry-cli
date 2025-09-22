### Sentry CLI API Endpoints

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/files/?cursor={}
  location: api::AuthenticatedApi::list_release_files_by_checksum
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/releases/{}/files/?cursor={}
  location: api::AuthenticatedApi::list_release_files_by_checksum
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/files/{}/?download=1
  location: api::AuthenticatedApi::get_release_file
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/releases/{}/files/{}/?download=1
  location: api::AuthenticatedApi::get_release_file
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/files/{}/
  location: api::AuthenticatedApi::get_release_file_metadata
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/releases/{}/files/{}/
  location: api::AuthenticatedApi::get_release_file_metadata
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/files/{}/
  location: api::AuthenticatedApi::delete_release_file
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/releases/{}/files/{}/
  location: api::AuthenticatedApi::delete_release_file
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/files/source-maps/?name={}
  location: api::AuthenticatedApi::delete_release_files
  region_aware: no
  absolute: no
  notes: 

- method: DELETE
  path_template: /organizations/{}/files/source-maps/?name={}
  location: api::AuthenticatedApi::delete_release_files
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /projects/{}/{}/releases/
  location: api::AuthenticatedApi::new_release
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/releases/
  location: api::AuthenticatedApi::new_release
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/
  location: api::AuthenticatedApi::update_release
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/releases/
  location: api::AuthenticatedApi::update_release
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/releases/{}/
  location: api::AuthenticatedApi::update_release
  region_aware: no
  absolute: no
  notes: 

- method: PUT
  path_template: /organizations/{}/releases/{}/
  location: api::AuthenticatedApi::set_release_refs
  region_aware: no
  absolute: no
  notes: 

- method: DELETE
  path_template: /projects/{}/{}/releases/{}/
  location: api::AuthenticatedApi::delete_release
  region_aware: no
  absolute: no
  notes: 

- method: DELETE
  path_template: /organizations/{}/releases/{}/
  location: api::AuthenticatedApi::delete_release
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/
  location: api::AuthenticatedApi::get_release
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/releases/{}/
  location: api::AuthenticatedApi::get_release
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /projects/{}/{}/releases/
  location: api::AuthenticatedApi::list_releases
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/releases/
  location: api::AuthenticatedApi::list_releases
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/commits/
  location: api::AuthenticatedApi::get_release_commits
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/releases/{}/commits/
  location: api::AuthenticatedApi::get_release_commits
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/releases/{}/previous-with-commits/
  location: api::AuthenticatedApi::get_previous_release_with_commits
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/releases/{}/deploys/
  location: api::AuthenticatedApi::create_deploy
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/releases/{}/deploys/
  location: api::AuthenticatedApi::list_deploys
  region_aware: no
  absolute: no
  notes: 

- method: PUT
  path_template: /projects/{}/{}/issues/?{qs}
  location: api::AuthenticatedApi::bulk_update_issue
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/chunk-upload/
  location: api::AuthenticatedApi::get_chunk_upload_options
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /projects/{}/{}/files/difs/assemble/
  location: api::AuthenticatedApi::assemble_difs
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/releases/{}/assemble/
  location: api::AuthenticatedApi::assemble_release_artifacts
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /organizations/{}/artifactbundle/assemble/
  location: api::AuthenticatedApi::assemble_artifact_bundle
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: /projects/{}/{}/files/preprodartifacts/assemble/
  location: api::AuthenticatedApi::assemble_build
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/files/proguard-artifact-releases
  location: api::AuthenticatedApi::associate_proguard_mappings
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/?cursor={}
  location: api::AuthenticatedApi::list_organizations
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/monitors/?cursor={}
  location: api::AuthenticatedApi::list_organization_monitors
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/projects/?cursor={}
  location: api::AuthenticatedApi::list_organization_projects
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /projects/{}/{}/events/?cursor={}
  location: api::AuthenticatedApi::list_organization_project_events
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/events/?{}
  location: api::AuthenticatedApi::fetch_organization_events
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/issues/?query={}&
  location: api::AuthenticatedApi::list_organization_project_issues
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/issues/?
  location: api::AuthenticatedApi::list_organization_project_issues
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/repos/?cursor={}
  location: api::AuthenticatedApi::list_organization_repos
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/events/{}/json/
  location: api::AuthenticatedApi::get_event
  region_aware: no
  absolute: no
  notes: 

- method: GET
  path_template: /organizations/{}/events/{}/json/
  location: api::AuthenticatedApi::get_event
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/files/dsyms/
  location: api::AuthenticatedApi::upload_dif_archive
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /projects/{}/{}/releases/{}/files/
  location: api::AuthenticatedApi::upload_release_file
  region_aware: no
  absolute: no
  notes: 

- method: UNKNOWN
  path_template: /organizations/{}/releases/{}/files/
  location: api::AuthenticatedApi::upload_release_file
  region_aware: no
  absolute: no
  notes: 

- method: POST
  path_template: https://<ingest-host>/api/<project>/envelope/
  location: api::EnvelopesApi::send_envelope
  region_aware: n/a
  absolute: yes
  notes: DSN-derived absolute URL; X-Sentry-Auth header

- method: GET
  path_template: https://release-registry.services.sentry.io/apps/sentry-cli/latest
  location: api::Api::get_latest_sentrycli_release
  region_aware: no
  absolute: yes
  notes: Release registry

- method: POST
  path_template: dynamic absolute URL (server-provided)
  location: api::Api::upload_chunks
  region_aware: no
  absolute: yes
  notes: URL provided by chunk-upload options; auth forced

- method: GET
  path_template: http(s)://<metro>/index.ios.bundle?platform=ios&dev=true
  location: commands::react_native::xcode::download_bundle
  region_aware: no
  absolute: yes
  notes: React Native dev server (Metro)

- method: GET
  path_template: http(s)://<metro>/index.ios.map?platform=ios&dev=true
  location: commands::react_native::xcode::download_sourcemap
  region_aware: no
  absolute: yes
  notes: React Native dev server (Metro)
