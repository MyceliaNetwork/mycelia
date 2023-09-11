mod component_service;

use std::{convert::Infallible, net::SocketAddr};

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
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tower::{util::BoxService, BoxError, Service, ServiceExt};

async fn handle_function_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

async fn handle_rpc_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

type ServiceCommandSink = tokio::sync::mpsc::Sender<()>;
type ServiceCommandSource = tokio::sync::mpsc::Receiver<()>;

async fn start_rpc_server(command_sink: ServiceCommandSink, socket_addr: SocketAddr) {
    let server = rpc_server::RpcServer::new(command_sink);
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
    socket_addr: SocketAddr,
) {
    type HttpFunctionComponent = BoxService<Request<Body>, Response<Body>, BoxError>;

    let store_producer = wasmtime_components::runtime::make_store_producer();
    let base_component = function_service::service::empty_base_function_component();

    let function_service_maker = new_function_service_maker(base_component, store_producer);

    // this is clearly abusing `map_request`
    // tower should have a way to "place a service"
    // in front of another one
    fn resolve_map_future(req: Request<Body>) -> HttpRequest {
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(map_request(req))
        })
    }

    fn map_function_service(service: FunctionComponentService) -> HttpFunctionComponent {
        let binding = service
            .map_request(resolve_map_future)
            .map_response(|resp| map_response(resp));
        binding.boxed()
    }

    let function_service_maker = function_service_maker
        .map_response(|v| map_function_service(v))
        .boxed_clone();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let mut function_service_maker = function_service_maker.boxed_clone();

    let component_host_server = Server::bind(&socket_addr)
        .serve(make_service_fn(move |_v: &AddrStream| {
            function_service_maker.call(())
        }))
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

    let _server_handle = tokio::spawn(async move {
        while let Some(_command) = command_stream.recv().await {
            // Handle Command
        }
        let _ = shutdown_tx.send(());
    });
    component_host_server.await;
    println!("component_host_server returned");
}

pub(crate) mod protos {
    tonic::include_proto!("development");
}

mod rpc_server {
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

mod cmd {
    use clap::Parser;

    /// Mycelia Development Server
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        /// path to a function component
        #[arg(short, long)]
        function_component: Option<String>,
    }
}

#[tokio::main]
async fn main() {
    let _args = crate::cmd::Args::parse();

    let rpc_host_addr = SocketAddr::from(([127, 0, 0, 1], 50051));
    let http_host_addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Command Sink / Source
    let (command_sink, command_source) = tokio::sync::mpsc::channel(10);

    let rpc_server_future = start_rpc_server(command_sink, rpc_host_addr);
    let http_server_future = start_development_server(command_source, http_host_addr);

    let rpc_server_task_handle = tokio::spawn(rpc_server_future);
    let http_service_task_handle = tokio::spawn(http_server_future);

    tokio::select! {
        _ = rpc_server_task_handle => {
            println!("rpc server task completed");
        }
        _ = http_service_task_handle => {
            println!("http server task completed");
        }
    }
}
