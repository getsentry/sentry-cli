use anyhow::{Context as _, Result};
use curl::easy::{Easy, IpResolve};

/// API client for Sentry.
pub struct Api {
    client: Easy,
}

impl Api {
    /// Creates a new API client.
    pub fn new() -> Result<Self> {
        let mut client = Easy::new();
        
        // Force IPv4 DNS resolution to work around glibc bug where statically-linked
        // binaries fail DNS resolution when IPv6 returns NXDOMAIN (even if IPv4 succeeds).
        // This commonly occurs in Docker containers on AWS.
        client.ip_resolve(IpResolve::V4)
            .context("Failed to configure IPv4 DNS resolution")?;
        
        Ok(Self { client })
    }
}