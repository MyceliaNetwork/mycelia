use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
// use tokio::net::TcpListener;
use tokio::net::TcpSocket;

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::Empty;

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the entire Mycelia project
    /// Shortcut for: `cargo build --workspace && cargo xtask build`
    Build,
    /// Start the Mycelia development server
    Start {
        /// The domain to listen on.
        /// Default: localhost
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "localhost")]
        domain: String,
        /// The port http server should bind to.
        /// Default: 3001
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "3001")]
        http_port: u16,

        /// The port rpc server should bind to
        /// Default: 50051
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "50051")]
        rpc_port: u16,

        /// Open the development server in your default browser after starting.
        /// Default: true
        /// Possible values: true, false
        #[clap(short, long, default_value = "true")]
        open_browser: bool,
        // TODO: add browser override list
    },
    /// Stop the Mycelia development server
    Stop {
        /// The domain to listen on.
        /// Default: localhost
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "localhost")]
        domain: String,

        /// The port rpc server should bind to
        /// Default: 50051
        /// TODO: add support to override (both here and in the development_server)
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    /// Deploy your Mycelia project
    Deploy,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn build() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["xtask", "build"])
        .status()?;

    if !status.success() {
        Err(format!(
            "`cargo xtask build` failed.

Status code: {}",
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

fn make_socket(addr: SocketAddr) -> TcpSocket {
    let socket = TcpSocket::new_v4().unwrap();
    socket.set_reuseaddr(true).unwrap(); // allow to reuse the addr both for connect and listen
    socket.set_reuseport(true).unwrap(); // same for the port
    socket.bind(addr).unwrap();
    socket
}

async fn is_peer_connected(addr: SocketAddr) -> bool {
    make_socket("127.0.0.1:3001".parse().unwrap())
        .connect(dbg!(addr))
        .await
        .is_ok()
}

async fn start(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> Result<(), DynError> {
    // TODO: might wanna move these prints to the development_server
    let http_addr = format!("http://{}:{}", domain, http_port);
    let rpc_addr = format!("http://{}:{}", domain, rpc_port);
    println!("HTTP development server listening on {}", http_addr);
    println!("RPC server listening on {}", rpc_addr);

    let () = if !is_peer_connected("127.0.0.1:3001".parse().unwrap()).await {
        println!("Peer not connected. Starting server");
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let _status = Command::new(cargo)
            .current_dir(project_root())
            .args(&[
                "run",
                "--package=development_server",
                "--",
                format!("--http-port={}", http_port).as_str(),
                format!("--rpc-port={}", rpc_port).as_str(),
            ])
            .stdout(Stdio::piped())
            .spawn();
    } else {
        println!("Peer already connected");
    };

    if *open_browser {
        let path = format!("http://{}:{}", domain, http_port);

        match open::that(&path) {
            Ok(()) => println!("Opened '{}' successfully.", path),
            Err(err) => eprintln!("An error occurred when opening '{}': {}", path, err),
        }
    }

    Ok(())
}

async fn stop(domain: &str, rpc_port: &u16) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_stop(domain, rpc_port).await {
        eprintln!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_stop(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    println!("Stopping development server");
    let address = format!("http://{}:{}", domain, rpc_port);
    let mut client = DevelopmentClient::connect(address).await?;

    let response = client.stop_server(Empty {}).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);

        // std::process::exit(-1);
    }

    Ok(())
}

async fn try_main() -> Result<(), DynError> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Build => {
            build()?;
        }
        Commands::Start {
            domain,
            http_port,
            rpc_port,
            open_browser,
        } => {
            start(domain, http_port, rpc_port, open_browser).await?;
        }
        Commands::Stop { domain, rpc_port } => {
            let _ = stop(domain, rpc_port).await;
        }
        Commands::Deploy => {
            println!("TODO: deploy");
        }
    }

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
