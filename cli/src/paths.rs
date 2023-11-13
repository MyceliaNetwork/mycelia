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

    pub fn dir_deployable() -> PathBuf {
        dir_project_root().join("deployable")
    }

    pub fn dir_deployable_backend(name: String) -> PathBuf {
        dir_deployable().join(name).join("backend")
    }
}
