// TODO move into own package
pub mod mycelia_core {
    use thiserror::Error;
    use tower::util::BoxService;

    #[derive(Error, Debug)]
    pub enum ServiceError {
        #[error("wait and try again")]
        NotReady,
        #[error("unknown failure")]
        Unknown,
    }
    pub type HostResourceIdProvider = BoxService<(), u32, ServiceError>;

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
    use std::collections::HashMap;

    use async_trait::async_trait;

    use tower::{util::BoxService, Service, ServiceExt};
    use wasmtime::component::{Component, Linker, Resource};
    use wasmtime::Store;

    use crate::{mycelia_core::HostResourceIdProvider, ClientServiceError};

    use self::bindgen::mycelia_alpha::http::interfaces::HostClient as HostClientInterface;
    use self::bindgen::mycelia_alpha::http::interfaces::{Client, ClientRequest, ClientResult};
    use self::bindgen::Command;

    // this helps with syntax completion
    mod bindgen {
        use wasmtime::component::*;

        bindgen!({
          world: "command",
          async: true
        });
    }

    use thiserror::Error;
    #[derive(Error, Debug)]
    pub enum ClientMakeError {
        #[error("resource not found")]
        NotFound,
        #[error("unknown failure")]
        Unknown,
    }

    pub type HostClient = BoxService<ClientRequest, ClientResult, ClientServiceError>;
    pub type HostClientMaker = BoxService<(), HostClient, ClientMakeError>;

    // TODO we need to support callbacks for drop
    pub struct HostClientResource {
        pub host_id_client: HostResourceIdProvider,
        pub client_maker: BoxService<(), HostClient, ClientMakeError>,
        pub clients: HashMap<u32, HostClient>,
    }

    impl HostClientResource {
        pub fn new(client_maker: HostClientMaker, host_id_client: HostResourceIdProvider) -> Self {
            Self {
                host_id_client,
                client_maker,
                clients: Default::default(),
            }
        }
    }

    #[async_trait]
    impl HostClientInterface for HostClientResource {
        async fn new(&mut self) -> anyhow::Result<Resource<Client>> {
            let rdy_client = self.host_id_client.ready().await?;
            Ok(Resource::new_own(rdy_client.call(()).await?))
        }

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

        fn drop(&mut self, _val: Resource<Client>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl bindgen::mycelia_alpha::http::types::Host for HostClientResource {}
    impl bindgen::mycelia_alpha::http::interfaces::Host for HostClientResource {}

    pub trait HostClientResourceMaker {
        fn new(&mut self) -> anyhow::Result<&mut HostClientResource>;
    }

    pub async fn setup_with_wasmtime<T: HostClientResourceMaker + Send>(
        store: &mut Store<T>,
        component: &Component,
        linker: &mut Linker<T>,
    ) -> anyhow::Result<()> {
        let _ = Command::add_to_linker::<T, HostClientResource>(linker, |v| {
            v.new().expect("failed to produce new host client resource")
        })?;

        let _ = bindgen::Command::instantiate_async(store, component, linker).await?;

        Ok(())
    }
}

pub mod providers {
    pub mod hyper {
        use std::collections::HashMap;

        use tower::{util::BoxService, ServiceExt};

        use crate::{host::{HostClientResourceMaker, HostClientResource}, mycelia_core::HostResourceIdProvider};
    }
}
