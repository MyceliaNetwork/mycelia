//! Core components primarily designed for building host wasm resource implementations.
//! The host-side code in this module is intended for use by the mycelia runtime but
//! can also be utilized in any wasmtime rust project.
//! This module provides functionalities to allow wasm guests to make HTTP requests.
//!
//! # Notes
//! - The guest-side code may need to be separated into its own crate.
//! - TLS needs to be enabled in the `providers::hyper` module's client.
//! - Error handling and its nuances need further refinement.

use thiserror::Error;
use tower::util::BoxService;

#[derive(Error, Debug)]
/// Errors associated with ID production.
pub enum IdProductionError {
    #[error("wait and try again")]
    NotReady,
    #[error("unknown failure")]
    Unknown,
}

/// A service that generates unique resource identifiers for wasm component resource providers.
/// TODO we need to provide a concrete implementation
pub type HostResourceIdProvider = BoxService<(), u32, IdProductionError>;
