// Usage:
//
// `cargo run --package cli`

use std::{
    env,
    io::Error,
    path::{Path, PathBuf},
    process::Command,
};

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// pub struct Args {
//     #[arg(short, long)]
//     path: String,
// }

fn main() -> Result<(), Error> {
    // let args = Args::parse();

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["run", "--package=development_server"])
        .status()?;

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
