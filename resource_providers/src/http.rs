//! Host side implementations for providing wasm HTTP client interactions.
//! This module provides the necessary structures and functionalities to manage
//! and provide HTTP client interactions for wasm guests. It leverages Tower
//! to offer precise control over request rates, client creation, etc.
//!
//! # Usage
//! This is intended to be used by the wasmtime hosts (like `mycelia`) to provide HTTP clients to wasm component guests.
//! It's pretty straightforward :)

use std::collections::HashMap;

use async_trait::async_trait;

use tower::{util::BoxService, Service, ServiceExt};
use wasmtime::component::{Component, Linker, Resource};
use wasmtime::Store;

use crate::core::HostResourceIdProvider;

use self::bindgen::mycelia_alpha::http::interfaces::Client;

/// Provides the host side implementation for a client resource.
///
/// This trait must be implemented by types that implement the host side of a client
/// resource.
use self::bindgen::mycelia_alpha::http::interfaces::HostClient as HostClientInterface;

use self::bindgen::Command;

pub use self::bindgen::mycelia_alpha::http::interfaces::{ClientRequest, ClientResult};
pub use self::bindgen::mycelia_alpha::http::types::{ClientResponse, Method};

// this helps with syntax completion as it isn't entirely obvious what's happening under the hood
// if you get really stuck you can use `cargo expand >> expanded.rs` to view the generated code
mod bindgen {
    use wasmtime::component::*;

    bindgen!({
      path: "../guest_crates/mycelia_http/wit",
      world: "command",
      async: true
    });
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientMakeError {
    #[error("wasm guest resource not found.")]
    NotFound,
}

#[derive(Error, Debug)]
pub enum HttpClientError {
    #[error("host client isn't ready. wait and try again.")]
    NotReady,
    #[error("unknown failure")]
    Unknown,
    #[error("client error")]
    ClientError { cause: String },
    #[error("guest produced a malformed request")]
    BadRequest,
}

/// Abstract service type defining an HttpClient
/// which can be provided by specific concrete implmentations
/// for example we have a `HyperHostClient`
pub type HostClient = BoxService<ClientRequest, ClientResult, HttpClientError>;

/// Abstract service type for a thing which produces new HostClients
/// for example see `providers::hyper::new_client_maker`
pub type HostClientMaker = BoxService<(), HostClient, ClientMakeError>;

/// Manages the association between guest wasm http clients and their host implementation instances.
pub struct HostClientResource {
    pub resource_id_provider: HostResourceIdProvider,
    pub client_maker: BoxService<(), HostClient, ClientMakeError>,
    pub clients: HashMap<u32, HostClient>,
}

impl HostClientResource {
    pub fn new(client_maker: HostClientMaker, resource_id_provider: HostResourceIdProvider) -> Self {
        Self {
            resource_id_provider,
            client_maker,
            clients: Default::default(),
        }
    }
}

#[async_trait]
impl HostClientInterface for HostClientResource {

    //! Creates a new guest HttpClient Resource storing its resource id
    async fn new(&mut self) -> anyhow::Result<Resource<Client>> {
        let rdy_client = self.resource_id_provider.ready().await?;
        let new_id = rdy_client.call(()).await?;

        let rdy_client = self.client_maker.ready().await?;
        let new_client = rdy_client.call(()).await?;

        if let Some(_) = self.clients.insert(new_id, new_client) {
            // print error. This is indicative of a bug in the upstream id provider client
        }

        Ok(Resource::new_own(new_id))
    }

    /// Attempts to make an HttpRequest using some resource
    async fn send(
        &mut self,
        guest_self: Resource<Client>,
        req: ClientRequest,
    ) -> anyhow::Result<ClientResult> {
        let id = guest_self.rep();
        let client = self.clients.get_mut(&id).ok_or(ClientMakeError::NotFound)?;
        let client = client.ready().await?;
        Ok(client.call(req).await?)
    }

    // Called when a resource falls out of scope
    // guest side
    fn drop(&mut self, val: Resource<Client>) -> anyhow::Result<()> {
        let id = val.rep();
        self.clients.remove(&id);
        Ok(())
    }
}

impl bindgen::mycelia_alpha::http::types::Host for HostClientResource {}
impl bindgen::mycelia_alpha::http::interfaces::Host for HostClientResource {}

pub trait HostClientResourceMaker {
    fn new(&mut self) -> anyhow::Result<&mut HostClientResource>;
}

// tell the linker how to provide access to the http client command world
// with the help of the `HostClientResourceMaker` trait
// and instantiate the http client command world
//
/// TODO: What the heck does "instantiate_async" actually do?
pub async fn setup_with_wasmtime<T: HostClientResourceMaker + Send>(
    store: &mut Store<T>,
    component: &Component,
    linker: &mut Linker<T>,
) -> anyhow::Result<()> {
    let _ = Command::add_to_linker::<T, HostClientResource>(linker, |v| {
        v.new().expect("failed to produce new host client resource")
    })?;

    let _ = Command::instantiate_async(store, component, linker).await?;

    Ok(())
}
