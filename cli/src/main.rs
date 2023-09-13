use clap::{Parser, Subcommand};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

type DynError = Box<dyn std::error::Error>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the mycelia development server
    Start,
}

fn start() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["run", "--package=development_server"])
        .status()?;

    if !status.success() {
        Err(format!(
            "
Starting development_server failed.

Command: `cargo run start`
Status code: {}",
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Start => {
            start()?;
        }
    }
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
