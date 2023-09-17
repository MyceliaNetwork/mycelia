use clap::{Parser, Subcommand};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::{EchoReply, EchoRequest, Empty, Success};

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
        #[clap(short, long, default_value = "127.0.0.1")]
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
        #[clap(short, long, default_value = "127.0.0.1")]
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

async fn server_listening(address: String) -> Result<(), DynError> {
    // TODO: return https://doc.rust-lang.org/stable/std/task/enum.Poll.html# ?
    // TODO: use rand string?
    let echo = "poll_dev_server".to_string();

    let client = DevelopmentClient::connect(address.clone());
    if let Err(e) = client.await {
        if e.to_string() == "transport error" {
            println!("Server not yet started");
            return Ok(());
        } else {
            return Err(e.into());
        }
    } else {
        // FIXME: prevent duplicate ::connect
        let mut client = DevelopmentClient::connect(address.clone()).await?;
        let message = EchoRequest {
            message: echo.clone(),
        };
        let request = tonic::Request::new(message);
        let response = client.echo(request).await?;

        match response.into_inner() {
            EchoReply { message } => {
                if message == echo {
                    println!("Development server already listening");
                    return Err("Development server already listening".into());
                } else {
                    println!("Error echoing message to RPC server");
                    return Ok(());
                }
            }
        }
    }
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

    // TODO: handle Error from server_listening
    if let Ok(_) = server_listening(rpc_addr.clone()).await {
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
            .spawn();

        println!("HTTP development server listening on {}", http_addr);
        println!("RPC server listening on {}", rpc_addr);
    }

    if *open_browser {
        match open::that(&http_addr) {
            Ok(()) => println!("Opened '{}' in your default browser.", http_addr),
            Err(err) => eprintln!("An error occurred when opening '{}': {}", http_addr, err),
        }
    } else {
        println!("You can reach the development server on {}", http_addr);
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
    let request = tonic::Request::new(Empty {});
    let response = client.stop_server(request).await?;

    match response.into_inner() {
        Success {} => {
            println!("Successfully stopped development server");
        }
    }

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
            let _ = start(domain, http_port, rpc_port, open_browser).await?;
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
