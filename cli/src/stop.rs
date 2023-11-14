#[allow(clippy::all)]
pub mod stop {
    use development::development_client::DevelopmentClient;
    use development::Empty;
    use log::{error, info, warn};
    use thiserror::Error;

    pub mod development {
        tonic::include_proto!("development");
    }

    pub async fn stop(ip: &str, rpc_port: &u16) {
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

    #[derive(Debug, Error)]
    enum StopError {
        #[error("client error. Cause: {cause:#?}")]
        ClientError { cause: String },
        #[error("client method error. Cause: {cause:#?}")]
        MethodError { cause: String },
    }
}
