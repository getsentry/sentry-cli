//! A collection of utilities for integration tests.

pub mod env;
mod mock_endpoint_builder;

pub use mock_endpoint_builder::{mock_endpoint, MockEndpointBuilder};
