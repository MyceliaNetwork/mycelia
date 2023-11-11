#[allow(clippy::all)]
pub mod paths {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    use semver::Version;

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

    pub fn dir_deployable_target() -> PathBuf {
        dir_project_root().join("deployable")
    }

    // pub fn dir_dist() -> PathBuf {
    //     dir_project_root().join("dist")
    // }

    pub fn dir_npm() -> PathBuf {
        dir_project_root().join("npm")
    }

    pub fn dir_npm_dist(version: Version) -> PathBuf {
        let path = format!("dist/{version}/npm/");
        let path = Path::new(path.as_str());
        dir_project_root().join(path)
    }

    // pub fn dir_npm_dist(tag: &String) -> PathBuf {
    //     let path = format!("dist/mycelia-{tag}/npm/mycelia");
    //     dir_project_root().join(path)
    // }

    // pub fn file_npm_package_manifest(tag: &String) -> PathBuf {
    //     dir_npm_dist(tag).join("package.json")
    // }

    // pub fn file_npm_info(tag: &String) -> PathBuf {
    //     dir_npm_dist(tag).join("info.json")
    // }
}
