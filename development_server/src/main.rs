mod component_service;

use std::{convert::Infallible, net::SocketAddr};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use tokio::select;
use tower::Service;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

async fn handle_function_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

async fn handle_rpc_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

type ServiceCommandSink = tokio::sync::mpsc::Sender<()>;
type ServiceCommandSource = tokio::sync::mpsc::Receiver<()>;

type ComponentService = BoxService<Request<Body>, Response<Body>, Infallible>;

struct BoxedServiceWrapper {
    inner: Pin<Box<dyn Future<Output = Result<Response<Body>, Infallible>> + Send>>,
}

impl Service<Request<Body>> for BoxedServiceWrapper {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Infallible>> + Send>>;

    // If for some reason you ever need to rate
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        Box::pin(async { Ok(Response::new("Boxed Service Response".into())) })
    }
}

async fn component_service_factory(_command_source: ServiceCommandSource) -> ComponentService {
    BoxService::new(BoxedServiceWrapper {
        inner: Box::pin(async { Ok(Response::new("hello world".into())) }),
    })
}

#[tokio::main]
async fn main() {
    // Component Host
    let component_host_addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_component_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_function_request)) });

    let component_host = Server::bind(&component_host_addr).serve(make_component_svc);

    let (command_pipe_tx, command_pipe_rx) = tokio::sync::mpsc::channel(10);

    // Rpc Host
    let rpc_host_addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    let make_rpc_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_rpc_request)) });

    let rpc_server = Server::bind(&rpc_host_addr).serve(make_rpc_svc);

    // Spawn Services
    let component_server = tokio::spawn(async {
        loop {
            tokio::select! {
                _ = component_host.fuse() => {
                    println!("operation timed out");
                    break;
                }
                _ = command_pipe_rx.recv() => {
                    println!("operation completed");
                }
            }
        }
    });

    let rpc_server = tokio::spawn(rpc_server);

    let _ = component_server.await;
    let _ = rpc_server.await;
}
