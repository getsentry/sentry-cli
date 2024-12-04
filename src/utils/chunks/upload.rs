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
}
