#[allow(clippy::all)]
use dotenvy;

use clap::{Parser, Subcommand};
use log::error;

mod build;
mod deploy;
mod new;
mod paths;
mod publish;
mod release;
mod start;
mod stop;

pub use crate::build::build::build;
pub use crate::deploy::deploy::deploy;
pub use crate::new::new::new;
pub use crate::publish::publish::publish;
pub use crate::release::release::release;
pub use crate::start::start::start;
pub use crate::stop::stop::stop;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

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

        /// The port rpc Versionshould bind to
        /// Default:Version
        #[clap(long, default_value = "50051")]
        rpc_port: u16,
    },
    /// Release a new MyceliaVersion
    Release,
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    dotenvy::dotenv()?;

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
            new().await?;
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
        Commands::Release => {
            release().await?;
        }
    }

    Ok(())
}
