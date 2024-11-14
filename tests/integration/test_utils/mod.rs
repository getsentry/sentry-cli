//! A collection of utilities for integration tests.

pub mod env;
mod mocking;

pub use mocking::{mock_endpoint, MockEndpointBuilder};
