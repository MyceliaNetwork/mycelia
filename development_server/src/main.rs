mod http_function_component;
mod rpc;

use std::net::SocketAddr;

use clap::Parser;

use log::{info, warn};

use http_function_component::*;
use rpc::*;

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
