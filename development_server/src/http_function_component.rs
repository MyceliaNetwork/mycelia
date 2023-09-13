use std::{net::SocketAddr, path::Path, sync::Arc};

use anyhow::anyhow;

use function_service::{
    service::{new_function_service_maker, FunctionComponentService},
    types::{HttpRequest, HttpResponse},
};
use hyper::service::Service as HyperService;
use hyper::{
    body::HttpBody, server::conn::AddrStream, service::make_service_fn, Body, Request, Response,
    Server,
};
use log::{info, trace, warn};

use tokio::sync::Mutex;
use tower::{
    util::{BoxCloneService, BoxService},
    BoxError, ServiceExt,
};

/// Map a hyper request to the mycelia::execution::HttpRequest type
pub(crate) async fn map_request(req: Request<Body>) -> HttpRequest {
    trace!("mapping incoming request {:#?}", &req);
    use function_service::types::Method;

    let method = match req.method().as_str() {
        "OPTIONS" => Method::Options,
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "DELETE" => Method::Delete,
        "HEAD" => Method::Head,
        "TRACE" => Method::Trace,
        "CONNECT" => Method::Connect,
        "PATCH" => Method::Patch,
        v => Method::Other(v.into()),
    };

    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                v.to_str().unwrap_or("not supported").to_string(),
            )
        })
        .collect();
    let uri = req.uri().to_string();

    // body::to_bytes is unsafe to use alone, don't do this in prod.
    let body = hyper::body::to_bytes(req.boxed()).await;
    let body = body.map(|v| v.to_vec()).unwrap_or(vec![]);

    HttpRequest {
        method,
        headers,
        body,
        uri,
    }
}

/// Map a mycelia::execution::HttpResponse type
pub(crate) fn map_response(response: HttpResponse) -> Response<Body> {
    trace!("mapping outgoing response {:#?}", response);
    let mut builder = Response::builder().status(response.status);

    let body = Body::from(response.body);

    for (k, v) in response.headers.into_iter() {
        builder = builder.header(k, v);
    }

    builder.body(body).expect("Failed to create a response")
}

/// helper function to faciliate reading the
/// async request body stream from sync context

// this is clearly abusing `map_request`
// tower should have a way to "place a service"
// in front of another one
pub(crate) fn map_http_request(req: Request<Body>) -> HttpRequest {
    tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(map_request(req))
    })
}

/// Decorates a FunctionComponentService with request response
/// mappers to allow it to handle incoming hyper http Request / Response
pub(crate) fn map_component_response(service: FunctionComponentService) -> HttpFunctionComponent {
    let binding = service
        .map_request(map_http_request)
        .map_response(|resp| map_response(resp));
    binding.boxed()
}

type HttpFunctionComponent = BoxService<Request<Body>, Response<Body>, BoxError>;

type HttpFunctionComponentMaker = BoxCloneService<(), HttpFunctionComponent, BoxError>;

/// Helper to produce new HttpFunctionComponentMakers
/// take note that this is where we're actually apply `map_component_response`
pub(crate) fn new_http_component_maker(
    component_maybe: Option<function_service::service::WasmComponent>,
) -> HttpFunctionComponentMaker {
    let store_producer = wasmtime_components::runtime::make_store_producer();
    let base_component =
        component_maybe.unwrap_or(function_service::service::empty_base_function_component());

    new_function_service_maker(base_component, store_producer)
        .map_response(map_component_response)
        .boxed_clone()
}

pub(crate) async fn start_development_server(
    mut command_stream: crate::rpc::ServiceCommandSource,
    socket_addr: SocketAddr,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let function_service_maker = Arc::new(Mutex::new(new_http_component_maker(None)));
    let function_service_maker_rpc_handle = function_service_maker.clone();

    let component_host_server = Server::bind(&socket_addr)
        .serve(make_service_fn(move |v: &AddrStream| {
            trace!("http connection {:#?}", v);
            let cloned_maker = function_service_maker.clone();
            async move {
                let mut maker = cloned_maker.lock().await;
                maker.call(()).await
            }
        }))
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

    // Spawn the manager event loop
    let _ = tokio::spawn(async move {
        let cloned_maker = function_service_maker_rpc_handle.clone();
        while let Some(command) = command_stream.recv().await {
            let _ = match command {
                crate::rpc::ServiceCommand::SwapFunctionComponent {
                    component_path,
                    reply,
                } => {
                    let component_path = Path::new(&component_path);
                    if !component_path.exists() || !component_path.is_file() {
                        let _ = reply.send(Err(anyhow!("Component path doesn't exist or isn't a file. Did you specify the correct path?")));
                    } else {
                        match wasmtime_components::runtime::new_component_from_path(
                            component_path.into(),
                        ) {
                            Ok(function_component) => {
                                info!("attempting to take lock on maker");
                                let mut locked_maker = cloned_maker.lock().await;
                                info!("received lock on maker. Attempting to swap with new function component maker");
                                let new_http_component_maker =
                                    new_http_component_maker(Some(function_component));
                                *locked_maker = new_http_component_maker;
                                let _ = reply.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = reply.send(Err(anyhow!("Failed to create a component from path. Did you specify a valid wasm32-wasi component?, Error {:#?}", e)));
                            }
                        }
                    }
                }
            };
        }
        let _ = shutdown_tx.send(());
    });
    let _ = component_host_server.await;

    warn!("component_host_server returned");
}
