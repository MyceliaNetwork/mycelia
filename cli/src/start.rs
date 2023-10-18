#[allow(clippy::all)]
pub mod start {
    use crate::paths::paths;
    use development::{EchoReply, EchoRequest};
    use log::{debug, error, info, trace, warn};
    use std::{env, future::Future, process::Stdio};
    use thiserror::Error;
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::{Child, ChildStderr, ChildStdout, Command},
        time::{Duration, Instant},
    };

    pub mod development {
        tonic::include_proto!("development");
    }
    use development::development_client::DevelopmentClient;
    struct DevelopmentServerClient {
        process: Child,
    }

    // We use the tonic crate to send an EchoRequest to the development_server through a gRPC address
    // The `just_started` argument is used to return ServerState::StartingUp in stead of
    // ServerState::NotStarted when "transport error" is returned by the gRPC client.
    async fn server_state(
        address: String,
        just_started: &bool,
    ) -> Result<ServerState, ServerError> {
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
                            error!("Unexpected EchoReply from RPC server. Message: {message}",);
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

    async fn spawn_dev_server_client(
        ip: &str,
        http_port: &u16,
        rpc_port: &u16,
        open_browser: &bool,
    ) {
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
            .current_dir(paths::project_root())
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

    pub async fn start(
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
            .current_dir(paths::project_root())
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

    #[derive(Error, Debug)]
    enum ServerError {
        #[error("development_server error. Cause: {cause:#?}")]
        ServerError { cause: String },
    }

    #[derive(Debug, Error)]
    enum StartError {
        #[error("server error. Cause: {cause:#?}")]
        ServerError { cause: String },
    }

    #[derive(PartialEq)]
    enum ServerState {
        NotStarted,
        StartingUp,
        Started,
    }

    #[derive(Debug, Error)]
    enum PollError {
        #[error("development_server error. Cause: {cause:#?}")]
        ServerError { cause: String },
        #[error("timeout")]
        Timeout,
    }
}
