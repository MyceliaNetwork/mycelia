#[allow(clippy::all)]
pub mod deploy {
    use crate::paths::paths;
    use crate::start::start::start;
    use crate::stop::stop::stop;
    use log::{error, info, warn};
    use thiserror::Error;
    use tokio::time::{Duration, Instant};
    pub mod development {
        tonic::include_proto!("development");
    }
    use development::development_client::DevelopmentClient;
    use development::{DeployReply, DeployRequest, EchoReply, EchoRequest};

    #[derive(PartialEq)]
    enum ServerState {
        NotStarted,
        StartingUp,
        Started,
    }

    /*
     * Usage:
     *
     * cargo run deploy --component=game
     *
     * This will take the file "./components/game.wasm" and deploy it.
     */
    pub async fn deploy(ip: &String, http_port: &u16, rpc_port: &u16, component: &String) {
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
        let path = paths::project_root().join(format!("components/{}.wasm", component));
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

    #[derive(Error, Debug)]
    enum ServerError {
        #[error("development_server error. Cause: {cause:#?}")]
        ServerError { cause: String },
    }

    #[derive(Debug, Error)]
    enum PollError {
        #[error("development_server error. Cause: {cause:#?}")]
        ServerError { cause: String },
        #[error("timeout")]
        Timeout,
    }

    #[derive(Debug, Error)]
    enum DeploymentError {
        #[error("path for component '{component}' not found. Path: {path}")]
        PathNotFound { component: String, path: String },
        #[error("client error. Cause: {cause:#?}")]
        ClientError { cause: String },
        #[error("deployment error. Cause: {cause:#?}")]
        DeploymentError { cause: String },
        #[error("server error")]
        ServerError,
    }
}
