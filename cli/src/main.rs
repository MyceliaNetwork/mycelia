use clap::{Parser, Subcommand};
use log::error;

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

mod deploy;
mod paths;
mod start;
mod stop;

pub use crate::deploy::deploy::deploy;
pub use crate::start::start::start;
pub use crate::stop::stop::stop;

pub mod development {
    tonic::include_proto!("development");
}

type DynError = Box<dyn Error>;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
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
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
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
            ip,
            http_port,
            rpc_port,
            open_browser,
            background,
        } => {
            start(ip, http_port, rpc_port, open_browser, background).await;
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

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("CARGO_MANIFEST_DIR not found")
        .to_path_buf()
}
