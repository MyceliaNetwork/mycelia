pub mod paths {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    pub fn project_root() -> PathBuf {
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .unwrap()
            .to_path_buf()
    }

    pub fn dir_target() -> PathBuf {
        project_root().join("target")
    }

    pub fn dir_components() -> PathBuf {
        project_root().join("components")
    }

    pub fn file_wasi_snapshot() -> PathBuf {
        project_root().join("wasi_snapshot_preview1.reactor.wasm.dev")
    }

    pub fn dir_guests() -> PathBuf {
        project_root().join("guests")
    }

    pub fn file_rustwrap() -> PathBuf {
        project_root().join("rustwrap.yaml")
    }
}
