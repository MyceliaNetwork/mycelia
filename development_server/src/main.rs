mod component_service;

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use function_service::{
    service::{new_function_service_maker, FunctionComponentService},
    types::{HttpRequest, HttpResponse},
};
use hyper::service::Service as HyperService;
use hyper::{
    body::HttpBody,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tower::{BoxError, Service, ServiceExt};

async fn handle_function_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

async fn handle_rpc_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

type ServiceCommandSink = tokio::sync::mpsc::Sender<()>;
type ServiceCommandSource = tokio::sync::mpsc::Receiver<()>;

async fn start_rpc_service(command_sink: ServiceCommandSink, socket_addr: SocketAddr) {
    let server = development_rpc_server::RpcServer::new(command_sink);
    let server = protos::development_server::DevelopmentServer::new(server);

    let _server = tonic::transport::Server::builder()
        .add_service(server)
        .serve(socket_addr)
        .await;
}

async fn map_request(req: Request<Body>) -> HttpRequest {
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

fn map_response(response: HttpResponse) -> Response<Body> {
    let mut builder = Response::builder().status(response.status);

    let body = Body::from(response.body);

    for (k, v) in response.headers.into_iter() {
        builder = builder.header(k, v);
    }

    builder.body(body).expect("Failed to create a response")
}

struct HttpService {
    component_service: FunctionComponentService,
}

impl Service<Request<Body>> for HttpService {
    type Response = Response<Body>;

    type Error = BoxError;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        todo!()
    }
}

async fn start_development_server(
    mut command_stream: ServiceCommandSource,
    _socket_addr: SocketAddr,
) {
    let store_producer = wasmtime_components::runtime::make_store_producer();
    let base_component = function_service::service::empty_base_function_component()
        .expect("Failed to get a base component. Cannot continue");

    let service_maker = Arc::new(Mutex::new(new_function_service_maker(
        base_component,
        store_producer,
    )));
    let component_host_addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|socket: &AddrStream| {
        let _remote_addr = socket.remote_addr();
        let service_maker = service_maker.clone();
        async move {
            let service_maker = service_maker.clone();

            Ok::<_, BoxError>(service_fn(move |req: Request<Body>| {
                let service_maker = service_maker.clone();

                async move {
                    let service_maker = service_maker.clone();
                    let mut service_maker = service_maker.lock().await;
                    let mut service = service_maker.ready().await?.call(()).await?;
                    let response = service.ready().await?.call(map_request(req).await).await?;
                    Ok::<_, BoxError>(map_response(response))
                }
            }))
        }
    });

    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let _component_host_server = Server::bind(&component_host_addr).serve(make_svc);
    //.with_graceful_shutdown(async move { let _ = shutdown_rx.await; });

    let _server_handle = tokio::spawn(async move {
        while let Some(_command) = command_stream.recv().await {
            // Handle Command
        }

        let _ = shutdown_tx.send(());
    });

    //component_host_server.await;
}

pub(crate) mod protos {
    tonic::include_proto!("development");
}

pub mod development_rpc_server {
    use crate::{
        protos::{development_server::Development, EchoReply, EchoRequest},
        ServiceCommandSink,
    };
    use tonic::Response;

    #[derive(Debug)]
    pub struct RpcServer {
        command_sink: ServiceCommandSink,
    }

    impl RpcServer {
        pub fn new(command_sink: ServiceCommandSink) -> Self {
            Self { command_sink }
        }
    }

    #[tonic::async_trait]
    impl Development for RpcServer {
        async fn echo(
            &self,
            request: tonic::Request<EchoRequest>,
        ) -> Result<Response<EchoReply>, tonic::Status> {
            Ok(Response::new(EchoReply {
                message: request.into_inner().message,
            }))
        }
    }
}

mod development_component_server {}

#[tokio::main]
async fn main() {
    // Component Host

    // Rpc Host
    let rpc_host_addr = SocketAddr::from(([127, 0, 0, 1], 50051));

    // Command Sink / Source
    let (command_sink, _command_source) = tokio::sync::mpsc::channel(10);
    let rpc_server_future = start_rpc_service(command_sink, rpc_host_addr);

    let rpc_server_task_handle = tokio::spawn(rpc_server_future);

    let _ = rpc_server_task_handle.await;
}
