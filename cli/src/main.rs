use clap::{Parser, Subcommand};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

// use tonic::{transport::Server, Request, Response, Status};

use cli::greeter_client::GreeterClient;
use cli::HelloRequest;

pub mod cli {
    tonic::include_proto!("cli");
}

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Subcommand)]
enum Commands {
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

fn start(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> Result<(), DynError> {
    // TODO: might wanna move these prints to the development_server
    println!(
        "Starting development server on http://{}:{}",
        domain, http_port
    );
    println!("Starting rpc server on http://{}:{}", domain, rpc_port);

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
    let mut client = GreeterClient::connect(address).await?;

    let request = tonic::Request::new(HelloRequest {
        name: "hello".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_main() -> Result<(), DynError> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Start {
            domain,
            http_port,
            rpc_port,
            open_browser,
        } => {
            start(domain, http_port, rpc_port, open_browser)?;
        }
        Commands::Stop { domain, rpc_port } => {
            // TODO: process Result
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
