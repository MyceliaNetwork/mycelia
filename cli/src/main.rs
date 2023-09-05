use std::{fs::File, io::Error};

use clap::Parser;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Wasm file to invoke
    #[arg(short, long)]
    path: String,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let file = File::open(args.path)?;

    Ok(())
}
