use std::{convert::Infallible, net::SocketAddr};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};

async fn handle_function_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

async fn handle_rpc_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("hello world".into()))
}

#[tokio::main]
async fn main() {
    // Function
    let function_host_addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_host_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_function_request)) });

    let host_server = Server::bind(&function_host_addr).serve(make_host_svc);

    // Rpc
    let rpc_host_addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    let make_rpc_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_rpc_request)) });

    let rpc_server = Server::bind(&rpc_host_addr).serve(make_rpc_svc);

    // Spawn Services
    let host_server = tokio::spawn(host_server);
    let rpc_server = tokio::spawn(rpc_server);

    let _ = host_server.await;
    let _ = rpc_server.await;
}
