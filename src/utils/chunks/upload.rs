use std::time::Duration;

use crate::api::ChunkServerOptions;

/// A trait representing options for chunk uploads.
pub trait ChunkOptions {
    /// Determines whether we need to strip debug_ids from the requests.
    /// When this function returns `true`, the caller is responsible for stripping
    /// the debug_ids from the requests, to maintain backwards compatibility with
    /// older Sentry servers.
    fn should_strip_debug_ids(&self) -> bool;

    /// Returns the organization that we are uploading to.
    fn org(&self) -> &str;

    /// Returns the project that we are uploading to.
    fn project(&self) -> &str;

    /// Returns whether we should wait for assembling to complete.
    fn should_wait(&self) -> bool;

    /// Returns the maximum wait time for the upload to complete.
    fn max_wait(&self) -> Duration;

    /// Returns the server options for the chunk upload.
    fn server_options(&self) -> &ChunkServerOptions;
}
