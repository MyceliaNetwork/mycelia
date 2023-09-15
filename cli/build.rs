// FIXME: this is a copy of /development_server/build.rs
//
// > it prob makes sense to eventualy get this into its own package.
// > But, for now feel free to just repeat the boilerplate in the CLI.

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // See https://github.com/hyperium/tonic/blob/master/examples/build.rs
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("development_descriptor.bin"))
        .compile(&["proto/development.proto"], &["proto"])
        .unwrap();

    tonic_build::compile_protos("proto/development.proto")?;
    Ok(())
}
