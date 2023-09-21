//! Provides a [hyper](https://crates.io/crates/hyper)-backed implementation of a host http client.
//! This module offers a concrete implementation of an http client for the host side
//! of wasm interactions. The client produced here is offered to wasm guests by
//! the mycelia runtime to allow them to make http requests.
//! TODO
//! * we MUST provide an example usage here
//! * developers should interact with the wasm client using the [wonderful http crate](https://crates.io/crates/http)
//!     so they're not using some weird types in their code

use std::{future::Future, pin::Pin};

use anyhow::anyhow;
use hyper::{body::HttpBody, Body, Method, Request};
use tower::{util::BoxService, Service, ServiceBuilder, ServiceExt};

use crate::http::{
    ClientRequest, ClientResponse, ClientResult, HostClientMaker, HostClientResource,
    HostClientResourceMaker, HttpClientError,
};

use crate::core::HostResourceIdProvider;

pub struct HyperClientResourceMaker {
    inner: HostClientResource,
}

/// `HostClientResourceMaker` by our core wasm resource provider framework (pattern?)
/// to provide the actual clients when requested by wasm
impl HostClientResourceMaker for HyperClientResourceMaker {
    fn new(&mut self) -> anyhow::Result<&mut HostClientResource> {
        let inner = &mut self.inner;
        Ok(inner)
    }
}

// Produce a new http client maker to be used in a host wasm runtime view
// which provides http component functionality to a guest running in the runtime
pub fn new(id_provider: HostResourceIdProvider) -> HyperClientResourceMaker {
    let client_maker = new_client_maker();

    let inner = HostClientResource::new(client_maker, id_provider);

    HyperClientResourceMaker { inner }
}

fn new_client_maker() -> HostClientMaker {
    let service = ServiceBuilder::new().service_fn(|_v: ()| async {
        let service = HyperHostClient;

        Ok(service.boxed())
    });

    BoxService::new(service)
}

struct HyperHostClient;

impl Service<ClientRequest> for HyperHostClient {
    type Response = ClientResult;

    type Error = HttpClientError; // TODO rename this error..

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ClientRequest) -> Self::Future {
        // TODO creating a new client for each request for now
        // in the future we should Arc<Mutex<Client>>
        // and implement poll correctly
        let client = hyper::Client::new();
        Box::pin(async move {
            let resp = client.request(req.try_into()?).await?;
            let (parts, mut data) = resp.into_parts();

            let body = read_body_stream(&mut data).await;
            let body = body.map_err(|e| HttpClientError::ClientError {
                cause: format!("failed to ready body {:#?}", e),
            })?;

            let status = parts.status;

            let headers = parts
                .headers
                .into_iter()
                .map(|(k, v)| {
                    (
                        k.map(|v| v.to_string()).unwrap_or("".to_string()),
                        v.to_str().unwrap().to_string(),
                    )
                })
                .collect();

            let r = ClientResult::Ok(ClientResponse {
                status: status.as_u16(),
                headers,
                body,
            });
            Ok(r)
        })
    }
}

impl TryInto<Request<Body>> for ClientRequest {
    type Error = HttpClientError;

    fn try_into(self) -> Result<Request<Body>, Self::Error> {
        let method = get_method(&self)?;

        let body = Body::from(self.body);

        let mut req = Request::builder().method(method).uri(self.uri);

        for (k, v) in self.headers {
            req = req.header(k, v);
        }

        req.body(body).map_err(|_| HttpClientError::BadRequest)
    }
}

impl From<hyper::Error> for HttpClientError {
    fn from(value: hyper::Error) -> Self {
        let cause = value.to_string();
        HttpClientError::ClientError { cause }
    }
}

// prevent a malicious tenant from consuming too much resources
static RESPONSE_LIMIT: usize = 5 * 1024 * 1024;

async fn read_body_stream(body: &mut hyper::Body) -> anyhow::Result<Vec<u8>> {
    let mut out: Vec<u8> = vec![];
    let mut size = 0;
    while let Some(response) = body.data().await {
        let bytes = response?;

        size += bytes.len();
        if size > RESPONSE_LIMIT {
            return Err(anyhow!(
                "attempted to stream too much data amount {} limit {}",
                size,
                RESPONSE_LIMIT
            ));
        }
        out.reserve(bytes.len());
        out.extend_from_slice(&bytes);
    }

    Ok(out)
}

fn get_method(req: &ClientRequest) -> Result<Method, HttpClientError> {
    use crate::http::Method as WasmHttpMethod;
    match &req.method {
        WasmHttpMethod::Get => Ok(Method::GET),
        WasmHttpMethod::Head => Ok(Method::HEAD),
        WasmHttpMethod::Post => Ok(Method::POST),
        WasmHttpMethod::Put => Ok(Method::PUT),
        WasmHttpMethod::Delete => Ok(Method::DELETE),
        WasmHttpMethod::Connect => Ok(Method::CONNECT),
        WasmHttpMethod::Options => Ok(Method::OPTIONS),
        WasmHttpMethod::Trace => Ok(Method::TRACE),
        WasmHttpMethod::Patch => Ok(Method::PATCH),
        WasmHttpMethod::Other(_) => Err(HttpClientError::BadRequest),
    }
}
