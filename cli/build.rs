// FIXME: this is a copy of /development_server/build.rs

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // See https://github.com/hyperium/tonic/blob/master/examples/build.rs
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("cli_descriptor.bin"))
        .compile(&["proto/cli.proto"], &["proto"])
        .unwrap();

    tonic_build::compile_protos("proto/cli.proto")?;
    Ok(())
}
