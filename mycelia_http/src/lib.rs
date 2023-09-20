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
    #[error("client error")]
    ClientError { cause : String },
    #[error("guest produced a malformed request")]
    BadRequest
}

pub mod host {
    use std::collections::HashMap;

    use async_trait::async_trait;

    use tower::{util::BoxService, Service, ServiceExt};
    use wasmtime::component::{Component, Linker, Resource};
    use wasmtime::Store;

    use crate::{mycelia_core::HostResourceIdProvider, ClientServiceError};

    use self::bindgen::mycelia_alpha::http::interfaces::HostClient as HostClientInterface;
    use self::bindgen::mycelia_alpha::http::interfaces::Client;
    use self::bindgen::Command;

    pub use self::bindgen::mycelia_alpha::http::interfaces::{ClientRequest, ClientResult};
    pub use self::bindgen::mycelia_alpha::http::types::{Method, ClientResponse};
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
            let new_id = rdy_client.call(()).await?;

            let rdy_client = self.client_maker.ready().await?;
            let new_client = rdy_client.call(()).await?;

            if let Some(_) = self.clients.insert(new_id, new_client) {
                // print error. This is indicative of a bug in the upstream id provider client
            }

            Ok(Resource::new_own(new_id))
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
        use std::{pin::Pin, future::Future, io::Bytes};

        use anyhow::anyhow;
        use hyper::{client::HttpConnector, Request, Body, Response, Method, body::HttpBody};
        use tower::{service_fn, ServiceBuilder, util::BoxService, Service, BoxError, ServiceExt};

        use crate::{host::{HostClientResourceMaker, HostClientResource, HostClientMaker, HostClient, ClientRequest, ClientResult, ClientResponse}, mycelia_core::HostResourceIdProvider, ClientServiceError};

        pub struct HyperClientResourceMaker {
            inner : HostClientResource
        }

        impl HostClientResourceMaker for HyperClientResourceMaker {
            fn new(&mut self) -> anyhow::Result<&mut crate::host::HostClientResource> {
                let inner = &mut self.inner;
                Ok(inner)
            }
        }

        pub fn new(id_provider : HostResourceIdProvider) -> HyperClientResourceMaker {
            let client_maker = new_client_maker();

            let inner = HostClientResource::new(client_maker, id_provider);

            HyperClientResourceMaker { inner }
        }

        fn new_client_maker() -> HostClientMaker {
            let service = ServiceBuilder::new().service_fn(|v : ()| async {
                let service = HyperHostClient;

                Ok(service.boxed())
            });

            BoxService::new(service)
        }

        struct HyperHostClient;

        impl Service<ClientRequest> for HyperHostClient {
            type Response = ClientResult;

            type Error = ClientServiceError; // TODO rename this error..

            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }

            fn call(&mut self, req: ClientRequest) -> Self::Future {
                // TODO creating a new client for each request for now
                // in the future we should Arc<Mutex<Client>>
                // and implement poll correctly
                let client = hyper::Client::new();
                Box::pin(async move {
                    let mut resp = client.request(req.try_into()?).await?;
                    let (parts, mut data) = resp.into_parts();

                    let body = read_body_stream(&mut data).await;
                    let body = body.map_err(|_| ClientServiceError::ClientError { cause: "failed to ready body".to_string() })?;

                    let status = parts.status;

                    let headers = parts.headers.into_iter()
                    .map(|(k, v)| (k.map(|v| v.to_string()).unwrap_or("".to_string()), v.to_str().unwrap().to_string())).collect();

                    let r = ClientResult::Ok(ClientResponse { status: status.as_u16(), headers, body });
                    Ok(r)
                })
            }
        }

        async fn make_request(client : &hyper::Client<HttpConnector>, request : ClientRequest) -> anyhow::Result<ClientResult> {
            todo!()
        }

        impl TryInto<Request<Body>> for ClientRequest {
            type Error = ClientServiceError;

            fn try_into(self) -> Result<Request<Body>, Self::Error> {
                let method = get_method(&self)?;

                let body = Body::from(self.body);

                let mut req = Request::builder()
                    .method(method)
                    .uri(self.uri);

                for (k ,v) in self.headers {
                    req = req.header(k, v);
                }

                req.body(body)
                .map_err(|_| ClientServiceError::BadRequest)
            }
        }

        impl From<hyper::Error> for ClientServiceError {
            fn from(value: hyper::Error) -> Self {
                let cause = value.to_string();
                ClientServiceError::ClientError { cause }
            }
        }

        static RESPONSE_LIMIT : usize = 5*1024*1024;

        async fn read_body_stream(body : &mut hyper::Body) -> anyhow::Result<Vec<u8>> {
            let mut out: Vec<u8> = vec![];
            let mut size = 0;
            while let Some(response) = body.data().await {
                let bytes = response?;

                size += bytes.len();
                if size > RESPONSE_LIMIT {
                    return Err(anyhow!("attempted to stream too much data {} limit {}", size, RESPONSE_LIMIT))
                }
                out.reserve(bytes.len());
                out.extend_from_slice(&bytes);
            }

            Ok(out)
        }

        fn get_method(req : &ClientRequest) -> Result<Method, ClientServiceError> {
            match &req.method {
                crate::host::Method::Get => Ok(Method::GET),
                crate::host::Method::Head => Ok(Method::HEAD),
                crate::host::Method::Post => Ok(Method::POST),
                crate::host::Method::Put => Ok(Method::PUT),
                crate::host::Method::Delete => Ok(Method::DELETE),
                crate::host::Method::Connect => Ok(Method::CONNECT),
                crate::host::Method::Options => Ok(Method::OPTIONS),
                crate::host::Method::Trace => Ok(Method::TRACE),
                crate::host::Method::Patch => Ok(Method::PATCH),
                crate::host::Method::Other(_) => Err(ClientServiceError::BadRequest),
            }
        }
    }
}
