mod component_service;

use std::{net::SocketAddr, path::Path, sync::Arc};

use anyhow::anyhow;
use clap::Parser;
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

use tokio::sync::{oneshot, Mutex};
use tower::{
    util::{BoxCloneService, BoxService},
    BoxError, Service, ServiceExt,
};

pub enum ServiceCommand {
    SwapFunctionComponent {
        component_path: String,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
}

type ServiceCommandSink = tokio::sync::mpsc::Sender<ServiceCommand>;
type ServiceCommandSource = tokio::sync::mpsc::Receiver<ServiceCommand>;

async fn start_rpc_server(command_sink: ServiceCommandSink, socket_addr: SocketAddr) {
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(protos::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let server = rpc_server::RpcServer::new(command_sink);
    let server = protos::development_server::DevelopmentServer::new(server);

    let _server = tonic::transport::Server::builder()
        .add_service(reflection)
        .add_service(server)
        .serve(socket_addr)
        .await;
}

async fn map_request(req: Request<Body>) -> HttpRequest {
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

fn map_response(response: HttpResponse) -> Response<Body> {
    trace!("mapping outgoing response {:#?}", response);
    let mut builder = Response::builder().status(response.status);

    let body = Body::from(response.body);

    for (k, v) in response.headers.into_iter() {
        builder = builder.header(k, v);
    }

    builder.body(body).expect("Failed to create a response")
}

// this is clearly abusing `map_request`
// tower should have a way to "place a service"
// in front of another one
fn map_http_request(req: Request<Body>) -> HttpRequest {
    tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(map_request(req))
    })
}

fn map_component_response(service: FunctionComponentService) -> HttpFunctionComponent {
    let binding = service
        .map_request(map_http_request)
        .map_response(|resp| map_response(resp));
    binding.boxed()
}

type HttpFunctionComponent = BoxService<Request<Body>, Response<Body>, BoxError>;

type HttpFunctionComponentMaker = BoxCloneService<(), HttpFunctionComponent, BoxError>;

fn new_http_component_maker(
    component_maybe: Option<function_service::service::WasmComponent>,
) -> HttpFunctionComponentMaker {
    let store_producer = wasmtime_components::runtime::make_store_producer();
    let base_component =
        component_maybe.unwrap_or(function_service::service::empty_base_function_component());

    new_function_service_maker(base_component, store_producer)
        .map_response(|v| map_component_response(v))
        .boxed_clone()
}

async fn start_development_server(
    mut command_stream: ServiceCommandSource,
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
                ServiceCommand::SwapFunctionComponent {
                    component_path,
                    reply,
                } => {
                    let component_path = Path::new(&component_path);
                    if !component_path.exists() || !component_path.is_file() {
                        reply.send(Err(anyhow!("Component path doesn't exist or isn't a file. Did you specify the correct path?")));
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

pub(crate) mod protos {
    tonic::include_proto!("development");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("development_descriptor");
}

mod rpc_server {
    use crate::{
        protos::{
            development_server::Development, DeployReply, DeployRequest, EchoReply, EchoRequest,
        },
        ServiceCommand, ServiceCommandSink,
    };
    use log::info;
    use tokio::sync::oneshot;
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

        async fn deploy_component(
            &self,
            request: tonic::Request<DeployRequest>,
        ) -> Result<Response<DeployReply>, tonic::Status> {
            info!("received deploy_component cmd");
            let request = request.into_inner();
            let component_path = request.component_path;

            let (reply, rx) = oneshot::channel();
            let cmd = ServiceCommand::SwapFunctionComponent {
                component_path,
                reply,
            };
            let _ = self.command_sink.send(cmd).await;

            if let Ok(Err(e)) = rx.await {
                return Err(tonic::Status::from_error(e.into()));
            }

            Ok(Response::new(DeployReply {
                message: "Ok".into(),
            }))
        }
    }
}

mod cmd {
    use clap::Parser;

    /// Mycelia Development Server
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        /// path to a function component
        #[arg(long)]
        pub function_component: Option<String>,

        /// port rpc server should bind to
        #[arg(long)]
        pub rpc_port: Option<u16>,

        /// port http server should bind to
        #[arg(long)]
        pub http_port: Option<u16>,
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("starting up");

    let args = crate::cmd::Args::parse();

    let rpc_host_addr = SocketAddr::from(([127, 0, 0, 1], args.rpc_port.unwrap_or(50051)));
    let http_host_addr = SocketAddr::from(([127, 0, 0, 1], args.http_port.unwrap_or(3001)));

    // Command Sink / Source
    let (command_sink, command_source) = tokio::sync::mpsc::channel(10);

    let rpc_server = start_rpc_server(command_sink, rpc_host_addr);
    let http_server = start_development_server(command_source, http_host_addr);

    let rpc_server = tokio::spawn(rpc_server);
    let http_server = tokio::spawn(http_server);

    tokio::select! {
        _ = rpc_server => {
            warn!("rpc server task completed");
        }
        _ = http_server => {
            warn!("http server task completed");
        }
    }
}
