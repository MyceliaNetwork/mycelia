// TODO move into own package
pub mod mycelia_core {
  use tower::util::BoxService;
  use thiserror::Error;

  #[derive(Error, Debug)]
  pub enum ServiceError {
    #[error("wait and try again")]
    NotReady,
    #[error("unknown failure")]
    Unknown,
  }
  pub type HostResourceIdProvider = BoxService<(),  u32, ServiceError>;

  pub struct MyceliaContext;
}

use thiserror::Error;
#[derive(Error, Debug)]
pub enum ClientServiceError {
    #[error("host client isn't ready. wait and try again.")]
    NotReady,
    #[error("unknown failure")]
    Unknown,
}

pub mod host {
    use anyhow::{anyhow, Context};
    use async_trait::async_trait;

    use tower::{util::BoxService, Service, ServiceExt};
    use wasmtime::Store;
    use wasmtime::component::{Resource, ResourceAny, Linker, Component};

    use crate::{mycelia_core::HostResourceIdProvider, ClientServiceError};

    use self::bindgen::Command;
    use self::bindgen::mycelia_alpha::http::interfaces::{ClientResult, ClientRequest, Client};
    use self::bindgen::mycelia_alpha::http::interfaces::HostClient;

    // this helps with syntax completion
    mod bindgen {
      use wasmtime::component::*;

      bindgen!({
        world: "command",
        async: true
      });
    }

    pub type HostClientService = BoxService<ClientRequest, ClientResult, ClientServiceError>;
    pub struct HostClientResource {
      client: HostClientService,
      host_id_client: HostResourceIdProvider
    }

    impl HostClientResource {
      pub fn new(client: HostClientService, host_id_client: HostResourceIdProvider) -> Self {
        Self {
          client, host_id_client
        }
      }
    }

    #[async_trait]
    impl HostClient for HostClientResource {
      async fn new(&mut self) -> anyhow::Result<Resource<Client>> {
        let rdy_client = self.host_id_client.ready().await?;
        Ok(Resource::new_own(rdy_client.call(()).await?))
      }

      async fn send(&mut self, _guest_self: Resource<Client>, req: ClientRequest) -> anyhow::Result<ClientResult> {
        let rdy_client = self.client.ready().await?;
        Ok(rdy_client.call(req).await?)
      }

      fn drop(&mut self, _val : Resource<Client>) -> anyhow::Result<()> {
        Ok(())
      }
    }

    impl bindgen::mycelia_alpha::http::types::Host for HostClientResource {

    }

    impl bindgen::mycelia_alpha::http::interfaces::Host for HostClientResource {

    }

    pub trait HostClientResourceMaker {
      fn new(&mut self) -> anyhow::Result<&mut HostClientResource>;
    }

    pub async fn instantiate_async<T: HostClientResourceMaker + Send>(store: &mut Store<T>, component: &Component, linker: &mut Linker<T>) -> anyhow::Result<()> {
      let _ = Command::add_to_linker::<T, HostClientResource>(linker, |v| {
        v.new().expect("failed to produce new host client resource")
      })?;

      let _ = bindgen::Command::instantiate_async(store, component, linker).await?;

      Ok(())
    }
}
