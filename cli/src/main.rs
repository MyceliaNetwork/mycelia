#[allow(clippy::all)]
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Input};
use log::{debug, error, info, trace, warn};
use tokio::sync::oneshot::channel;

use std::{
    env,
    error::Error,
    fs,
    future::Future,
    path::{Path, PathBuf},
    process::Stdio,
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout, Command},
    time::{Duration, Instant},
};
use version_compare::{compare, Cmp};

pub mod development {
    tonic::include_proto!("development");
}
use development::development_client::DevelopmentClient;
use development::{DeployReply, DeployRequest, EchoReply, EchoRequest, Empty};

#[derive(Debug, Error)]
enum StartError {
    #[error("server error. Cause: {cause:?}")]
    ServerError { cause: String },
}

#[derive(Debug, Error)]
enum StopError {
    #[error("client error. Cause: {cause:?}")]
    ClientError { cause: String },
    #[error("client method error. Cause: {cause:?}")]
    MethodError { cause: String },
}

#[derive(PartialEq)]
enum ServerState {
    NotStarted,
    StartingUp,
    Started,
}

#[derive(Error, Debug)]
enum ServerError {
    #[error("development_server error. Cause: {cause:?}")]
    ServerError { cause: String },
}

#[derive(Debug, Error)]
enum PollError {
    #[error("development_server error. Cause: {cause:?}")]
    ServerError { cause: String },
    #[error("timeout")]
    Timeout,
}

#[derive(Debug, Error)]
enum DeploymentError {
    #[error("path for component '{component:?}' not found. Path: {path:?}")]
    PathNotFound { component: String, path: String },
    #[error("client error. Cause: {cause:?}")]
    ClientError { cause: String },
    #[error("deployment error. Cause: {cause:?}")]
    DeploymentError { cause: String },
    #[error("server error")]
    ServerError,
}

#[derive(Debug, Error)]
enum NewProjectError {
    #[error("npm/npx not found")]
    NpmNotFound,
    #[error("npx create-next-app failed. Status code: {status:}")]
    CreateNextAppFailed { status: i32 },
}

#[derive(Debug, Error)]
enum ReleaseError {
    #[error(
        "argument `--version {version_input:?}` is lower than the current version {version_current:?}"
    )]
    VersionLowerThanCurrent {
        version_input: String,
        version_current: String,
    },
    #[error(
        "argument `--version {version_input:?}` equal to the current version {version_current:?}"
    )]
    VersionEqualToCurrent {
        version_input: String,
        version_current: String,
    },
    #[error("Version comparison error")]
    VersionComparisonError,
    #[error(
        "`cargo xtask --version {version:?}` failed.

        Status code: {status:?}"
    )]
    XtaskReleaseFailed { version: String, status: i32 },
}

type DynError = Box<dyn Error>;

struct DevelopmentServerClient {
    process: Child,
}

#[derive(Debug, Parser)]
#[command(
    author,
    about,
    version,
    long_about = None,
    propagate_version = true,
    disable_version_flag = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the entire Mycelia project
    /// Shortcut for: `cargo build --workspace && cargo xtask build`
    Build,
    /// Start the Mycelia development server
    Start {
        /// The ip to listen on.
        /// Default: 127.0.0.1
        #[clap(short, long, default_value = "127.0.0.1")]
        ip: String,

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
    /// Start a new boilerplate Mycelia project
    New,
    /// Stop the Mycelia development server
    Stop {
        /// The ip to listen on.
        /// Default: 127.0.0.1
        #[clap(short, long, default_value = "127.0.0.1")]
        ip: String,

        /// The port rpc server should bind to
        /// Default: 50051
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    /// Deploy your Mycelia component
    Deploy {
        /// The component inside `./components/` which is being deployed.
        #[clap(long)]
        component: String,

        /// The ip to listen on.
        /// Default: localhost
        #[clap(short, long, default_value = "127.0.0.1")]
        ip: String,

        /// The port http server should bind to.
        /// Default: 3001
        #[clap(long, default_value = "3001")]
        http_port: u16,

        /// The port rpc server should bind to
        /// Default: 50051
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    /// Release a new Mycelia version
    Release {
        /// The new Mycelia version
        #[clap(long)]
        version: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::init();

    if let Err(error) = try_main().await {
        error!("{error:?}");
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
            ip,
            http_port,
            rpc_port,
            open_browser,
            background,
        } => {
            start(ip, http_port, rpc_port, open_browser, background).await;
        }
        Commands::New => {
            let _ = new().await;
        }
        Commands::Stop { ip, rpc_port } => {
            stop(ip, rpc_port).await;
        }
        Commands::Deploy {
            ip,
            http_port,
            rpc_port,
            component,
        } => {
            deploy(ip, http_port, rpc_port, component).await;
        }
        Commands::Release { version } => {
            release(version).await;
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

// We use the tonic crate to send an EchoRequest to the development_server through a gRPC address
// The `just_started` argument is used to return ServerState::StartingUp in stead of
// ServerState::NotStarted when "transport error" is returned by the gRPC client.
async fn server_state(address: String, just_started: &bool) -> Result<ServerState, ServerError> {
    let payload = "cli::server_state()".to_string();
    match DevelopmentClient::connect(address).await {
        Ok(mut client) => {
            let message = EchoRequest {
                message: payload.clone(),
            };
            let request = tonic::Request::new(message);
            let response = client.echo(request).await;

            match response.unwrap().into_inner() {
                EchoReply { message } => {
                    if message == payload.to_string() {
                        warn!("Development server already listening");
                        return Ok(ServerState::Started);
                    } else {
                        error!(
                            "Unexpected EchoReply message from RPC server. Message: {}",
                            message
                        );
                    }
                    return Ok(ServerState::StartingUp);
                }
            };
        }
        Err(err) => match err.to_string().as_str() {
            "transport error" => {
                return match *just_started {
                    true => Ok(ServerState::StartingUp),
                    false => Ok(ServerState::NotStarted),
                };
            }
            err => {
                return Err(ServerError::ServerError { cause: err.into() });
            }
        },
    };
}

async fn poll_server_state(
    ip: &str,
    rpc_port: &u16,
    just_started: &bool,
) -> Result<ServerState, PollError> {
    let start = Instant::now();
    let timeout = Duration::from_secs(10);

    loop {
        let rpc_addr = format!("http://{}:{}", ip, rpc_port);
        let state = server_state(rpc_addr, just_started).await;

        match state {
            Ok(ServerState::StartingUp) => {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if start.elapsed() > timeout {
                    return Err(PollError::Timeout);
                }
            }
            Ok(ServerState::Started) => return Ok(ServerState::Started),
            Ok(ServerState::NotStarted) => return Ok(ServerState::NotStarted),
            Err(ServerError::ServerError { cause }) => {
                return Err(PollError::ServerError { cause })
            }
        };
    }
}

async fn spawn_dev_server_client(ip: &str, http_port: &u16, rpc_port: &u16, open_browser: &bool) {
    let http_addr = format!("http://{}:{}", ip, http_port);
    let rpc_addr = format!("http://{}:{}", ip, rpc_port);
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

{status:#?}"
        );
    });

    info!("Started development server");
    debug!("HTTP development server listening on {http_addr}");
    debug!("RPC server listening on {rpc_addr}");

    if *open_browser {
        let server_state = poll_server_state(ip, rpc_port, &true).await;
        if server_state.is_ok_and(|s| s == ServerState::Started) {
            match open::that(&http_addr) {
                Ok(()) => debug!("Opened '{http_addr}' in your default browser."),
                Err(error) => error!(
                    "An error occurred when opening '{http_addr}'.
Error:

{error:?}"
                ),
            }
        }
    }

    info!("You can reach the development server on {}", http_addr);

    return tokio::select! {
        // Wait for the handlers to exit. Currently this will never happen
        wait = wait => trace!("wait {wait:#?}"),
        task_handle = task_handle => {
            match task_handle {
                Ok(_) => std::process::exit(-1),
                Err(error) => error!("task_handle error {error:?}")
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
    trace!("proc id: {proc_id}",);

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
                Ok(Some(string)) => trace!("handle_stdout: {string}",),
                Ok(None) => trace!("handle_stdout: None"),
                Err(error) => trace!("handle_stdout: {error:#?}"),
            }
        }
    });

    let handle_stderr = tokio::spawn(async move {
        let mut reader = stderr_reader.lines();
        loop {
            match reader.next_line().await {
                Ok(Some(string)) => info!("handle_stderr: {}", string),
                Ok(None) => trace!("handle_stderr: None"),
                Err(error) => info!("handle_stderr: {error:#?}"),
            }
        }
    });
    // Wait for a handler to exit. You already spawned the handlers, you don't need to spawn them again.
    // I'm using select instead of join so we can see any errors immediately.
    tokio::select! {
        recv = handle_stdout => info!("recv: {:#?}", recv),
        send = handle_stderr => info!("send: {:#?}", send),
    };
}

async fn start(
    ip: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
    background: &bool,
) {
    if let Err(error) = try_start(ip, http_port, rpc_port, open_browser, background).await {
        error!("{error:?}");

        std::process::exit(-1);
    }
}

async fn try_start(
    ip: &String,
    http_port: &u16,
    rpc_port: &u16,
    open_browser: &bool,
    background: &bool,
) -> Result<(), StartError> {
    info!("Starting development server");
    let rpc_addr = format!("http://{}:{}", ip, rpc_port);

    match server_state(rpc_addr, &false).await {
        Ok(_) => match *background {
            false => spawn_dev_server_client(ip, http_port, rpc_port, open_browser).await,
            true => start_background(http_port, rpc_port).await,
        },
        Err(err) => {
            return Err(StartError::ServerError {
                cause: err.to_string(),
            });
        }
    }

    Ok(())
}

async fn start_background(http_port: &u16, rpc_port: &u16) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let _ = Command::new(cargo)
        .env("RUST_LOG", "off")
        .current_dir(project_root())
        .args(&[
            "run",
            "--quiet",
            "--package=development_server",
            "--",
            format!("--http-port={}", http_port).as_str(),
            format!("--rpc-port={}", rpc_port).as_str(),
        ])
        .stdout(Stdio::null())
        .spawn()
        .expect("Unable to spawn development_server");
}

async fn new() -> Result<(), DynError> {
    if let Err(error) = try_new().await {
        error!("{error:?}");

        std::process::exit(-1);
    }

    return Ok(());
}

async fn try_new() -> Result<(), NewProjectError> {
    info!("Creating new Mycelia project");

    let _ = fs::create_dir_all(&deployable_target());

    let app_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Your app name")
        .interact_text()
        .unwrap();

    if let Err(error) = scaffold_next(app_name).await {
        return Err(error);
    }

    return Ok(());
}

async fn scaffold_next(app_name: String) -> Result<(), NewProjectError> {
    let _ = fs::create_dir_all(&deployable_target());
    let (send, recv) = channel::<()>();
    let cargo = env::var("NPX").unwrap_or_else(|_| "npx".to_string());
    let mut npx_cmd = Command::new(cargo)
        .current_dir(&deployable_target())
        .args(&[
            "create-next-app@latest",
            app_name.as_str(),
            "--name=blabli",
            "--typescript",
            "--eslint",
            "--tailwind",
            "--src-dir",
            "--no-app",
            "--no-import-alias",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("npx create-next-app failed");

    tokio::spawn(async move { send.send(()) });
    tokio::select! {
        wait = npx_cmd.wait() => trace!("wait {wait:#?}"),
        recv = recv => {
            match recv {
                Ok(_) => {
                    println!("OKAY");
                }
                Err(error) => {
                    println!("🪵 [main.rs:592]~ token ~ \x1b[0;32merror\x1b[0m = {}", error);
                }
            }
        }
    }

    return Ok(());
}

async fn stop(ip: &str, rpc_port: &u16) {
    if let Err(error) = try_stop(ip, rpc_port).await {
        error!("{error:?}");

        std::process::exit(-1);
    }
}

async fn try_stop(ip: &str, rpc_port: &u16) -> Result<(), StopError> {
    info!("Stopping development server");
    let address = format!("http://{}:{}", ip, rpc_port);
    let client = DevelopmentClient::connect(address.clone()).await;
    match client {
        Ok(mut client) => {
            let request = tonic::Request::new(Empty {});
            let response = client.stop_server(request).await;
            if response.is_err() {
                return Err(StopError::MethodError {
                    cause: response.unwrap_err().to_string(),
                });
            }

            match response.unwrap().into_inner() {
                Empty {} => {
                    warn!("Stopped development server");
                    return Ok(());
                }
            }
        }
        Err(err) => {
            return match err.to_string().as_str() {
                "transport error" => {
                    warn!("Server not yet started");
                    return Ok(());
                }
                err => Err(StopError::ClientError {
                    cause: err.to_string(),
                }),
            };
        }
    };
}

/*
 * Usage:
 *
 * cargo run deploy --component=game
 *
 * This will take the file "./components/game.wasm" and deploy it.
 */
async fn deploy(ip: &String, http_port: &u16, rpc_port: &u16, component: &String) {
    if let Err(error) = try_deploy(ip, http_port, rpc_port, component).await {
        error!("{error:?}");

        std::process::exit(-1);
    }
}

async fn try_deploy(
    ip: &String,
    http_port: &u16,
    rpc_port: &u16,
    component: &String,
) -> Result<(), DeploymentError> {
    let path = project_root().join(format!("components/{}.wasm", component));
    if !path.exists() {
        return Err(DeploymentError::PathNotFound {
            component: component.clone(),
            path: path.clone().display().to_string(),
        });
    }

    let server_state = poll_server_state(ip, rpc_port, &false).await;
    if server_state
        .as_ref()
        .is_ok_and(|s| s == &ServerState::NotStarted)
    {
        let _ = start(ip, http_port, rpc_port, &false, &true).await;
        let _ = poll_server_state(ip, rpc_port, &true).await;
    };

    if server_state.is_ok() {
        let address = format!("http://{}:{}", ip, rpc_port);
        let client = DevelopmentClient::connect(address.clone()).await;
        match client {
            Ok(mut client) => {
                let message = DeployRequest {
                    component_path: path.clone().display().to_string(),
                };
                let request = tonic::Request::new(message);
                let response = client
                    .deploy_component(request)
                    .await
                    .expect("Deploy component failed");
                match response.into_inner() {
                    DeployReply { message } => {
                        if server_state.unwrap() == ServerState::NotStarted {
                            stop(ip, rpc_port).await;
                        }
                        if message == "Ok".to_string() {
                            info!("Deployed component to path: {}", path.display());
                            return Ok(());
                        } else {
                            return Err(DeploymentError::DeploymentError { cause: message });
                        }
                    }
                };
            }
            Err(err) => {
                if server_state.unwrap() == ServerState::NotStarted {
                    stop(ip, rpc_port).await;
                }
                return Err(DeploymentError::ClientError {
                    cause: err.to_string(),
                });
            }
        }
    }

    return Err(DeploymentError::ServerError);
}

async fn release(version_arg_val: &String) {
    if let Err(error) = try_release(version_arg_val).await {
        error!("{error:#}");

        std::process::exit(-1);
    }
}

async fn try_release(version_arg_val: &String) -> Result<(), ReleaseError> {
    info!("Releasing new Mycelia version {version_arg_val:}");

    let version_current: &str = env!("CARGO_PKG_VERSION");
    let version_comparison = compare(version_arg_val.clone(), version_current);
    if version_comparison.is_err() {
        return Err(ReleaseError::VersionComparisonError);
    }
    match version_comparison {
        Ok(Cmp::Lt) => {
            return Err(ReleaseError::VersionLowerThanCurrent {
                version_input: version_arg_val.clone(),
                version_current: version_current.to_string(),
            });
        }
        Ok(Cmp::Eq) => {
            return Err(ReleaseError::VersionEqualToCurrent {
                version_input: version_arg_val.clone(),
                version_current: version_current.to_string(),
            });
        }
        _ => {}
    }

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let release_process = std::process::Command::new(cargo)
        .current_dir(project_root())
        .args(&["xtask", "release", "--version", version_arg_val])
        .status()
        .expect("Unable to spawn release process");

    return match release_process.code() {
        Some(0) => Ok(()),
        Some(code) => Err(ReleaseError::XtaskReleaseFailed {
            version: version_arg_val.clone(),
            status: code,
        }),
        None => Err(ReleaseError::XtaskReleaseFailed {
            version: version_arg_val.clone(),
            status: -1,
        }),
    };
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("CARGO_MANIFEST_DIR not found")
        .to_path_buf()
}

fn deployable_target() -> PathBuf {
    project_root().join("deployable")
}
