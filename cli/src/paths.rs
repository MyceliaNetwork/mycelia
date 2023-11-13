#[allow(clippy::all)]
pub mod paths {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    pub fn dir_project_root() -> PathBuf {
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .unwrap()
            .to_path_buf()
    }

    pub fn dir_target() -> PathBuf {
        dir_project_root().join("target")
    }

    pub fn dir_components() -> PathBuf {
        dir_project_root().join("components")
    }

    pub fn file_wasi_snapshot() -> PathBuf {
        dir_project_root().join("wasi_snapshot_preview1.reactor.wasm.dev")
    }

    pub fn dir_guests() -> PathBuf {
        dir_project_root().join("guests")
    }
}
