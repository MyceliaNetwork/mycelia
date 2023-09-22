use clap::{Parser, Subcommand};
use log::{debug, error, info, trace, warn};

use std::{
    env,
    error::Error,
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{Duration, Instant},
};

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::{DeployReply, DeployRequest, EchoReply, EchoRequest, Empty};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("host client isn't ready. wait and try again.")]
    NotReady,
    #[error("not yet started")]
    NotStarted,
    #[error("already started")]
    AlreadyStarted,
    #[error("client error")]
    ClientError { cause: String },
    #[error("unknown failure")]
    Unknown,
}

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

        /// Run the development server in the background.
        /// Default: false
        /// Possible values: true, false
        #[clap(short, long, default_value = "false")]
        background: bool,
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
    Deploy {
        /// The component inside `./components/` which is being deployed.
        #[clap(long)]
        component: String,

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
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
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
            background,
        } => {
            let _ = start(domain, http_port, rpc_port, open_browser, background).await;
        }
        Commands::Stop { domain, rpc_port } => {
            let _ = stop(domain, rpc_port).await;
        }
        Commands::Deploy {
            domain,
            http_port,
            rpc_port,
            component,
        } => {
            let _ = deploy(domain, http_port, rpc_port, component).await;
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

async fn server_listening(address: String) -> Result<(), ClientError> {
    let payload = "poll_dev_server";

    match DevelopmentClient::connect(address).await {
        Ok(mut client) => {
            let message = EchoRequest {
                message: payload.to_string(),
            };
            let request = tonic::Request::new(message);
            let response = client.echo(request).await;

            match response.unwrap().into_inner() {
                EchoReply { message } => {
                    if message == payload.to_string() {
                        warn!("Development server already listening");
                        return Err(ClientError::AlreadyStarted);
                    } else {
                        error!("Error echoing message to RPC server");
                        return Err(ClientError::NotReady);
                    }
                }
            };
        }
        Err(err) => match err.to_string().as_str() {
            "transport error" => return Ok(()),
            err => return Err(ClientError::ClientError { cause: err.into() }),
        },
    };
}

async fn poll_server_listening(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    tokio::time::sleep(Duration::from_secs(1)).await;

    loop {
        let rpc_addr = format!("http://{}:{}", domain, rpc_port);
        match server_listening(rpc_addr).await {
            Err(ClientError::NotReady) => {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if start.elapsed() > timeout {
                    return Err("Timeout waiting for server to start")?;
                }
            }
            Err(ClientError::AlreadyStarted) => return Ok(()),
            // TODO: this should be an Err
            Err(ClientError::NotStarted) => return Ok(()),
            Err(ClientError::ClientError { cause }) => {
                return Err(cause)?;
            }
            Err(ClientError::Unknown) => return Err("Unknown error")?,
            Ok(_) => return Ok(()),
        };
    }
}

async fn spawn_client(
    domain: &str,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
    background: &bool,
) {
    let http_addr = format!("http://{}:{}", domain, http_port);
    let rpc_addr = format!("http://{}:{}", domain, rpc_port);
    let (mut client, wait) = start_development_server(http_port, rpc_port);

    // Spin off child process to make sure it can make process on its own
    // while we read its output
    let task_handle = tokio::spawn(async move {
        let status = client
            .process
            .wait()
            .await
            .expect("development server process encountered an error");

        error!(
            "Process exited. Cannot continue. Error:

{:#?}",
            status
        );
    });

    info!("Started development server");
    debug!("HTTP development server listening on {}", http_addr);
    debug!("RPC server listening on {}", rpc_addr);

    if *open_browser {
        let _ = poll_server_listening(domain, rpc_port).await;
        match open::that(&http_addr) {
            Ok(()) => debug!("Opened '{}' in your default browser.", http_addr),
            Err(err) => error!("An error occurred when opening '{}': {}", http_addr, err),
        }
    }

    info!("You can reach the development server on {}", http_addr);

    if *background {
        return;
    }
    return tokio::select! {
        // Wait for the handlers to exit. Currently this will never happen
        wait = wait => trace!("wait {:?}", wait),
        task_handle = task_handle => {
            match task_handle {
                Ok(_) => std::process::exit(-1),
                Err(e) => error!("task_handle error {:?}", e)
            }
        }
    };
}

fn start_development_server(
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
    trace!("proc id: {}", proc_id);

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
                Ok(Some(string)) => trace!("handle_stdout: {}", string),
                Ok(None) => trace!("handle_stdout: None"),
                Err(e) => trace!("handle_stdout: {:?}", e),
            }
        }
    });

    let handle_stderr = tokio::spawn(async move {
        let mut reader = stderr_reader.lines();
        loop {
            match reader.next_line().await {
                Ok(Some(string)) => info!("handle_stderr: {}", string),
                Ok(None) => info!("handle_stderr: None"),
                Err(e) => info!("handle_stderr: {:?}", e),
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
    background: &bool,
) -> Result<(), DynError> {
    info!("Starting development server");
    let rpc_addr = format!("http://{}:{}", domain, rpc_port);

    match server_listening(rpc_addr).await {
        Ok(_) => spawn_client(domain, http_port, rpc_port, open_browser, background).await,
        Err(e) => error!("Listening Error: {:?}", e),
    }

    Ok(())
}

async fn stop(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    if let Err(e) = try_stop(domain, rpc_port).await {
        error!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_stop(domain: &str, rpc_port: &u16) -> Result<(), DynError> {
    info!("Stopping development server");
    let address = format!("http://{}:{}", domain, rpc_port);
    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(mut client) => {
            let request = tonic::Request::new(Empty {});
            let response = client.stop_server(request).await?;

            match response.into_inner() {
                Empty {} => warn!("Stopped development server"),
            }
        }
        Err(err) => {
            return match err.to_string().as_str() {
                "transport error" => {
                    warn!("Server not yet started");
                    return Ok(());
                }
                err => *Box::new(Err(err.into())),
            };
        }
    };

    Ok(())
}

/*
 * Usage:
 *
 * cargo run deploy --component-path="./components/meh.wasm"
 */
async fn deploy(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    component: &String,
) -> Result<(), DynError> {
    if let Err(e) = try_deploy(domain, http_port, rpc_port, component).await {
        error!("{}", e);

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_deploy(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    component: &String,
) -> Result<(), DynError> {
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

    poll_server_listening(domain, rpc_port).await;

    println!("🪵 [main.rs:463]~ token ~ \x1b[0;32mpoll_server_listening\x1b[0m = ");

    let address = format!("http://{}:{}", domain, rpc_port);
    let path = format!("./components/{}.wasm", component);

    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(mut client) => {
            let message = DeployRequest {
                component_path: path.clone(),
            };
            let request = tonic::Request::new(message);
            let response = client.deploy_component(request).await?;

            match response.into_inner() {
                DeployReply { message } => {
                    if message == "Ok".to_string() {
                        info!("Deployed component to path: {}", path);
                        return Ok(());
                    } else {
                        error!("Error deploying component. Error: {:?}", message);
                        return Err("Error deploying component. Error".into());
                    }
                }
            };
        }
        Err(e) => {
            error!("Deployment Error: {:?}", e);
            return Err("Deployment Error".into());
        }
    }
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("CARGO_MANIFEST_DIR not found")
        .to_path_buf()
}

/// old school development server start

async fn trigger(domain: &String, http_port: &u16, rpc_port: &u16, open_browser: &bool) {
    let (_client, tx_send, _tx_recv, wait) =
        start_server_deploy(domain, http_port, rpc_port, open_browser);

    tx_send.send("initialize".as_bytes().to_vec()).ok();

    // Wait for the handlers to exit. Currently this will never happen
    wait.await
}

fn start_server_deploy(
    domain: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
) -> (
    DevelopmentServerClient,
    UnboundedSender<Vec<u8>>,
    UnboundedSender<Vec<u8>>,
    impl Future<Output = ()>,
) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut process = Command::new(cargo)
        .current_dir(project_root())
        .args(&[
            "run",
            "--package=development_server",
            "--",
            format!("--http-port={}", http_port).as_str(),
            format!("--rpc-port={}", rpc_port).as_str(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("Unable to spawn process");

    let proc_id = process.id().expect("Unable to fetch process id");
    println!("proc id: {}", proc_id);

    let stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();
    // let stderr = process.stderr.take().unwrap();

    let writer = BufWriter::new(stdin);
    let reader = BufReader::new(stdout);

    let (tx_send, rx_send) = unbounded_channel::<Vec<u8>>();
    let (tx_recv, rx_recv) = unbounded_channel::<Vec<u8>>();

    println!("setting up tx,rx");
    // This future waits for handlers to exit, we don't want to await it here.
    // Return it instead so the caller can await it.
    let wait = setup_listeners_deploy(reader, writer, rx_send, rx_recv);

    // Send initialize request
    println!("initialize request");
    tx_send.send("initialize".as_bytes().to_vec()).ok();
    let client = DevelopmentServerClient { process };

    (client, tx_send, tx_recv, wait)
}

async fn setup_listeners_deploy(
    mut reader: BufReader<ChildStdout>,
    mut writer: BufWriter<ChildStdin>,
    mut rx_send: UnboundedReceiver<Vec<u8>>,
    mut rx_recv: UnboundedReceiver<Vec<u8>>,
) {
    let handle_recv = tokio::spawn(async move {
        loop {
            // Wait until a message is available instead of constantly polling for a message.
            match rx_recv.recv().await {
                Some(data) => {
                    println!("rx_recv got: {:?}", String::from_utf8(data));

                    let mut buf = String::new();
                    let _ = reader.read_line(&mut buf);
                }
                None => {
                    println!("rx_recv: quitting loop");
                    break;
                }
            }
        }
    });

    let handle_send = tokio::spawn(async move {
        loop {
            // Wait until a message is available instead of constantly polling for a message.
            match rx_send.recv().await {
                Some(data) => {
                    println!("rx_send: got something");
                    let str = String::from_utf8(data).expect("invalid data for parse");
                    println!("rx_send: sending to LSP: {:?}", str);

                    _ = writer.write_all(str.as_bytes());
                }
                None => {
                    println!("rx_recv: quitting loop");
                    break;
                }
            }
        }
    });

    // Wait for a handler to exit. You already spawned the handlers, you don't need to spawn them again.
    // I'm using select instead of join so we can see any errors immediately.
    tokio::select! {
       send = handle_send => println!("{send:?}"),
       recv = handle_recv => println!("{recv:?}"),
    };
}
