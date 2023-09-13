use clap::{Parser, Subcommand};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start the Mycelia development server
    Start {
        /// The address to listen on.
        /// Default: localhost
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "localhost")]
        address: String,
        /// The port to listen on.
        /// Default: 3001
        /// TODO: add support to override (both here and in the development_server)
        #[clap(short, long, default_value = "3001")]
        port: u16,
        /// Open the development server in your default browser after starting.
        /// Default: true
        /// Possible values: true, false
        #[clap(short, long, default_value = "true")]
        open_browser: bool,
        // TODO: add browser override list
    },
    /// Stop the Mycelia development server
    Stop,
    /// Deploy your Mycelia project
    Deploy,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn start(address: &String, port: &u16, open_browser: &bool) -> Result<(), DynError> {
    println!("Starting development server on http://{}:{}", address, port);

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["run", "--package=development_server"])
        .stdout(Stdio::piped())
        .spawn();

    //     if !status.success() {
    //         Err(format!(
    //             "
    // Starting development_server failed.

    // Command: `cargo run start`
    // Status code: {}",
    //             status.code().unwrap()
    //         ))?;
    //     } else {
    //         println!("Development server started");
    //     }

    if *open_browser {
        let path = format!("http://{}:{}", address, port);

        match open::that(&path) {
            Ok(()) => println!("Opened '{}' successfully.", path),
            Err(err) => eprintln!("An error occurred when opening '{}': {}", path, err),
        }
    }

    Ok(())
}

fn stop() -> Result<(), DynError> {
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
        Commands::Start {
            address,
            port,
            open_browser,
        } => {
            start(address, port, open_browser)?;
        }
        Commands::Stop => {
            stop()?;
        }
        Commands::Deploy => {
            println!("TODO: deploy");
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
