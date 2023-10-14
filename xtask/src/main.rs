#[allow(clippy::all)]
use log::{error, info};
use std::env;

mod build;
mod publish;
mod release;

pub use crate::build::build::build;
pub use crate::publish::publish::publish;
pub use crate::release::release::release;

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::init();

    if let Err(error) = try_main().await {
        error!("{error:#}");
        std::process::exit(-1);
    }
}

type DynError = Box<dyn std::error::Error>;

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);

    match task.as_deref() {
        Some("build") => build()?,
        Some("release") => release().await?,
        Some("publish") => publish().await?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    info!(
        "Tasks:

build    Build all guests and components
release  Release a new version
publish  Publish a new version"
    )
}
