use std::net::SocketAddr;

use log::info;

use tokio::sync::oneshot;

pub(crate) mod protos {
    tonic::include_proto!("development");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("development_descriptor");
}

pub enum ServiceCommand {
    SwapFunctionComponent {
        component_path: String,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
    StopServer {
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
}

pub type ServiceCommandSink = tokio::sync::mpsc::Sender<ServiceCommand>;
pub type ServiceCommandSource = tokio::sync::mpsc::Receiver<ServiceCommand>;

use crate::protos::{
    development_server::Development, DeployReply, DeployRequest, EchoReply, EchoRequest, Empty
};

#[derive(Debug)]
pub(crate) struct RpcServer {
    command_sink: ServiceCommandSink,
}

impl RpcServer {
    pub fn new(command_sink: ServiceCommandSink) -> Self {
        Self { command_sink }
    }
}

#[tonic::async_trait]
impl Development for RpcServer {
    async fn echo(
        &self,
        request: tonic::Request<EchoRequest>,
    ) -> Result<tonic::Response<EchoReply>, tonic::Status> {
        Ok(tonic::Response::new(EchoReply {
            message: request.into_inner().message,
        }))
    }

    async fn deploy_component(
        &self,
        request: tonic::Request<DeployRequest>,
    ) -> Result<tonic::Response<DeployReply>, tonic::Status> {
        info!("received deploy_component cmd");
        let request = request.into_inner();
        let component_path = request.component_path;

        let (reply, rx) = oneshot::channel();
        let cmd = ServiceCommand::SwapFunctionComponent {
            component_path,
            reply,
        };
        let _ = self.command_sink.send(cmd).await;

        if let Ok(Err(e)) = rx.await {
            return Err(tonic::Status::from_error(e.into()));
        }

        Ok(tonic::Response::new(DeployReply {
            message: "Ok".into(),
        }))
    }

    async fn stop_server(
        &self,
        _request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<Empty>, tonic::Status> {
        let (reply, rx) = oneshot::channel();

        let _ = self.command_sink.send(ServiceCommand::StopServer { reply }).await;
        match rx.await {
            Ok(Ok(())) => Ok(tonic::Response::new(Empty{})),
            _ => Err(tonic::Status::from_error("Failed to stop server".into())),
        }
    }
}

pub(crate) async fn start_rpc_server(command_sink: ServiceCommandSink, socket_addr: SocketAddr) {
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(protos::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let server = RpcServer::new(command_sink);
    let server = protos::development_server::DevelopmentServer::new(server);

    let _server = tonic::transport::Server::builder()
        .add_service(reflection)
        .add_service(server)
        .serve(socket_addr)
        .await;
}
