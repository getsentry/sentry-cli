use std::{cmp, time::Duration};

use crate::api::ChunkServerOptions;

/// A struct representing options for chunk uploads.
pub struct ChunkOptions<'a> {
    server_options: ChunkServerOptions,
    org: &'a str,
    project: &'a str,

    /// The maximum wait time for the upload to complete.
    /// If set to zero, we do not wait for the upload to complete.
    /// If the server_options.max_wait is set to a smaller nonzero value,
    /// we use that value instead.
    max_wait: Duration,
}

impl<'a> ChunkOptions<'a> {
    pub fn new(server_options: ChunkServerOptions, org: &'a str, project: &'a str) -> Self {
        Self {
            server_options,
            org,
            project,
            max_wait: Duration::ZERO,
        }
    }

    /// Set the maximum wait time for the assembly to complete.
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    pub fn should_strip_debug_ids(&self) -> bool {
        self.server_options.should_strip_debug_ids()
    }

    pub fn org(&self) -> &str {
        self.org
    }

    pub fn project(&self) -> &str {
        self.project
    }

    pub fn should_wait(&self) -> bool {
        !self.max_wait().is_zero()
    }

    pub fn max_wait(&self) -> Duration {
        // If the server specifies a max wait time (indicated by a nonzero value),
        // we use the minimum of the user-specified max wait time and the server's
        // max wait time.
        match self.server_options.max_wait {
            0 => self.max_wait,
            server_max_wait => cmp::min(self.max_wait, Duration::from_secs(server_max_wait)),
        }
    }

    pub fn server_options(&self) -> &ChunkServerOptions {
        &self.server_options
    }
}
