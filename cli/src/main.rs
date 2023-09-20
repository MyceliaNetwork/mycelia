use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};

use std::{
    env,
    error::Error,
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout, Command},
    time::{Duration, Instant},
};

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::{EchoReply, EchoRequest, Empty, Success};

type DynError = Box<dyn Error>;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

struct DevelopmentServerClient {
    process: Child,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the entire Mycelia project
    /// Shortcut for: `cargo build --workspace && cargo xtask build`
    Build,
    /// Start the Mycelia development server
    Start {
        /// The domain to listen on.
        /// Default: localhost
        #[clap(short, long, default_value = "127.0.0.1")]
        domain: String,
        /// The port http server should bind to.
        /// Default: 3001
        #[clap(long, default_value = "3001")]
        http_port: u16,

        /// The port rpc server should bind to
        /// Default: 50051
        #[clap(long, default_value = "50051")]
        rpc_port: u16,

        /// Open the development server in your default browser after starting.
        /// Default: true
        /// Possible values: true, false
        #[clap(short, long, default_value = "true")]
        open_browser: bool,
    },
    /// Stop the Mycelia development server
    Stop {
        /// The domain to listen on.
        /// Default: localhost
        #[clap(short, long, default_value = "127.0.0.1")]
        domain: String,

        /// The port rpc server should bind to
        /// Default: 50051
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    /// Deploy your Mycelia project
    Deploy,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "debug")
    }

    env_logger::init();

    if let Err(e) = try_main().await {
        error!("{}", e);
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
            let _ = start(domain, http_port, rpc_port, open_browser).await;
        }
        Commands::Stop { domain, rpc_port } => {
            let _ = stop(domain, rpc_port).await;
        }
        Commands::Deploy => {
            let _ = deploy().await;
        }
    }

    Ok(())
}

fn build() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = std::process::Command::new(cargo)
        .current_dir(project_root())
        .args(&["xtask", "build"])
        .status()?;

    if !status.success() {
        Err(format!(
            "`cargo xtask build` failed.

Status code: {}",
            status.code().expect("Build failed: no status")
        ))?;
    }

    Ok(())
}

async fn server_listening(address: String) -> Result<(), DynError> {
    let payload = "poll_dev_server";
    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(mut client) => {
            let message = EchoRequest {
                message: payload.to_string(),
            };
            let request = tonic::Request::new(message);
            let response = client.echo(request).await?;

            match response.into_inner() {
                EchoReply { message } => {
                    if message == payload.to_string() {
                        debug!("Development server already listening");
                        return Err("Development server already listening".into());
                    } else {
                        error!("Error echoing message to RPC server");
                        return Ok(());
                    }
                }
            };
        }
        Err(err) => match err.to_string().as_str() {
            "transport error" => {
                debug!("Server not yet started");
                return Ok(());
            }
            err => *Box::new(Err(err.into())),
        },
    }
}

async fn poll_server_listening() -> Result<(), DynError> {
    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        if let Err(_) = server_listening("http://127.0.0.1:50051".to_string()).await {
            warn!("Development server listening");
            return Ok(());
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        if start.elapsed() > timeout {
            Err("Timeout waiting for server to start")?;
        }
    }
}

async fn trigger(http_port: &u16, rpc_port: &u16) {
    let (client, wait) = start_server(http_port, rpc_port);

    // Spin off child process to make sure it can make process on its own
    // while we read its output

    let mut process = client.process;

    let task_handle = tokio::spawn(async move {
        let status = process
            .wait()
            .await
            .expect("development server process encountered an error");
        info!(
            "Process exited {:#?}.
Cannot continue.",
            status
        );
    });

    tokio::select! {
        // Wait for the handlers to exit. Currently this will never happen
        wait = wait =>  error!("wait {:?}", wait),
        task_handle = task_handle => {
            match task_handle {
                Ok(_) => std::process::exit(-1),
                Err(e) => error!("task_handle error {:?}", e)
            }
        }
    }
}

fn start_server(
    http_port: &u16,
    rpc_port: &u16,
) -> (DevelopmentServerClient, impl Future<Output = ()>) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let log_level = env::var("RUST_LOG").expect("env::var RUST_LOG not set");
    let mut process = Command::new(cargo)
        .env("RUST_LOG", log_level)
        .current_dir(project_root())
        .args(&[
            "run",
            "--package=development_server",
            "--",
            format!("--http-port={}", http_port).as_str(),
            format!("--rpc-port={}", rpc_port).as_str(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("Unable to spawn process");

    let proc_id = process.id().expect("Unable to fetch process id");
    info!("proc id: {}", proc_id);

    let stdout = process.stdout.take().unwrap();
    let stdout_reader = BufReader::new(stdout);

    let stderr = process.stderr.take().unwrap();
    let stderr_reader = BufReader::new(stderr);

    // This future waits for handlers to exit, we don't want to await it here.
    // Return it instead so the caller can await it.
    let wait = setup_listeners(stdout_reader, stderr_reader);

    // Send initialize request
    debug!("initialize request");
    let client = DevelopmentServerClient { process };

    (client, wait)
}

async fn setup_listeners(
    stdout_reader: BufReader<ChildStdout>,
    stderr_reader: BufReader<ChildStderr>,
) {
    let handle_stdout = tokio::spawn(async move {
        let mut reader = stdout_reader.lines();
        loop {
            match reader.next_line().await {
                Ok(Some(string)) => info!("handle_recv: {}", string),
                Ok(None) => info!("handle_recv: None"),
                Err(e) => error!("handle_recv: {:?}", e),
            }
        }
    });

    let handle_stderr = tokio::spawn(async move {
        let mut reader = stderr_reader.lines();
        loop {
            match reader.next_line().await {
                Ok(Some(string)) => info!("handle_send: {}", string),
                Ok(None) => info!("handle_send: None"),
                Err(e) => error!("handle_send: {:?}", e),
            }
        }
    });
    // Wait for a handler to exit. You already spawned the handlers, you don't need to spawn them again.
    // I'm using select instead of join so we can see any errors immediately.
    tokio::select! {
        recv = handle_stdout => info!("recv: {:?}", recv),
        send = handle_stderr => info!("send: {:?}", send),
    };
}

async fn start(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> Result<(), DynError> {
    let http_addr = format!("http://{}:{}", domain, http_port);
    let rpc_addr = format!("http://{}:{}", domain, rpc_port);

    // TODO: handle Error from server_listening
    if let Ok(_) = server_listening(rpc_addr.clone()).await {
        trigger(http_port, rpc_port).await;

        debug!("HTTP development server listening on {}", http_addr);
        debug!("RPC server listening on {}", rpc_addr);
    }

    if *open_browser {
        poll_server_listening().await?;
        match open::that(&http_addr) {
            Ok(()) => debug!("Opened '{}' in your default browser.", http_addr),
            Err(err) => error!("An error occurred when opening '{}': {}", http_addr, err),
        }
    } else {
        debug!("You can reach the development server on {}", http_addr);
    }

    Ok(())
}

async fn stop(domain: &str, rpc_port: &u16) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = try_stop(domain, rpc_port).await {
        error!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_stop(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    debug!("Stopping development server");
    let address = format!("http://{}:{}", domain, rpc_port);
    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(mut client) => {
            // let mut client = DevelopmentClient::connect(address.clone()).await?;
            let request = tonic::Request::new(Empty {});
            let response = client.stop_server(request).await?;

            match response.into_inner() {
                Success {} => {
                    debug!("Successfully stopped development server");
                }
            }
        }
        Err(err) => {
            return match err.to_string().as_str() {
                "transport error" => {
                    debug!("Server not yet started");
                    return Ok(());
                }
                err => *Box::new(Err(err.into())),
            };
        }
    };

    Ok(())
}

async fn deploy() -> Result<(), DynError> {
    todo!("deploy")
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("CARGO_MANIFEST_DIR not found")
        .to_path_buf()
}
