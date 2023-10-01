use log::error;

use std::{
    cmp::Ordering,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;
use version_compare::{compare_to, Cmp};

#[derive(Debug, Error)]
enum BuildError {
    #[error(
        "
    `cargo build --workspace` failed.

Status code: {status}"
    )]
    Workspace { status: i32 },

    #[error(
        "Build wasm '{guest_name:?}' failed.

Command: `cargo build --target wasm32-wasi --release --package {guest_name:?}`
Guest path: '{guest_path:?}'
Status code: {status}"
    )]
    Wasm {
        guest_name: String,
        guest_path: PathBuf,
        status: i32,
    },

    #[error("wasm guest file '{guest_name:?}' for '{guest_path:?}' does not exist")]
    GuestFileNonExistent {
        guest_name: String,
        guest_path: PathBuf,
    },

    #[error(
        "Build component '{guest_name:?}' failed.

Command: `wasm-tools component new {path_wasm_guest:?} --adapt {path_wasi_snapshot:?} -o {dir_components:?}`
Guest path: '{guest_path:?}'
Status code: {status:?}"
    )]
    CommandFailed {
        guest_name: String,
        path_wasm_guest: PathBuf,
        path_wasi_snapshot: PathBuf,
        dir_components: PathBuf,
        guest_path: PathBuf,
        status: i32,
    },

    #[error("wasi snapshot file '{guest_name:?}' for '{guest_path:?}' does not exist")]
    WasiSnapshotFileNonExistent {
        guest_name: String,
        guest_path: PathBuf,
    },

    #[error("component output directory '{dir:?}' for '{guest_name:?}' does not exist")]
    DirComponentsNonExistent { dir: PathBuf, guest_name: String },
}
#[derive(Debug, Error)]
enum ReleaseError {
    #[error("missing --version argument")]
    MissingVersion,
    #[error(
        "argument --version ({version_input:?}) is lower than current version ({version_current:?}"
    )]
    VersionLowerThanCurrent {
        version_current: String,
        version_input: String,
    },
    #[error("building workspace failded. Status code: {status:?}")]
    BuildWorkspace { status: i32 },
}

type DynError = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);

    // println!("{:#?}", env::args());

    match task.as_deref() {
        Some("build") => build()?,
        Some("release") => match release().await {
            Err(e) => {
                return Err(Box::new(e));
            }
            Ok(_) => {}
        },
        _ => print_help(),
    }
    return Ok(());
}

fn print_help() {
    eprintln!(
        "Tasks:

wasm            build wasm using wasm32-wasi target
components      build components using wasm-tools
"
    )
}

#[derive(Debug, Clone)]
struct Guest {
    path: PathBuf,
    name: String,
    name_output: String,
}

impl Guest {
    fn new(path: PathBuf, name: &str, name_output: Option<&str>) -> Self {
        let name_output = name_output.unwrap_or(name).to_string();
        let name = name.to_string();
        return Self {
            path,
            name,
            name_output,
        };
    }
}

// 1. Read all contents of the ./guests/ directory
// 2. Filter out all non-directories (like README.md)
// 3. Map the remaining paths to Guest structs containing its:
//   - path: used for build
//   - name: used for Error messages
//   - name_output: used for build when defined in `name_map`
// 4. Order the items by priority. Because packages like `function` should be built last
fn guests() -> Vec<Guest> {
    let dir = fs::read_dir(&dir_guests()).unwrap();
    let name_map = HashMap::from([("js_function", "mycelia_guest_function")]);
    let priority = vec!["*".to_string(), "mycelia_guest_function".to_string()];

    let mut guests_filtered = dir
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_dir())
        .map(|p| {
            let name = p.strip_prefix(&dir_guests()).unwrap().to_str().unwrap();
            let name_output = name_map.get(name);
            return Guest::new(p.clone(), name, name_output.copied());
        })
        .collect::<Vec<_>>();

    guests_filtered.sort_by(|a, b| {
        let a = priority.iter().position(|p| p == &a.name).unwrap_or(0);
        let b = priority.iter().position(|p| p == &b.name).unwrap_or(0);
        if a > b {
            return Ordering::Greater;
        } else if a < b {
            return Ordering::Less;
        } else {
            return Ordering::Equal;
        }
    });

    return guests_filtered;
}

fn build() -> Result<(), DynError> {
    fs::create_dir_all(&dir_target())?;
    fs::create_dir_all(&dir_components())?;

    for guest in guests() {
        build_wasm(&guest)?;
        build_component(&guest)?;
    }
    build_workspace()?;

    Ok(())
}

fn build_workspace() -> Result<(), BuildError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--workspace"])
        .status();

    if !status.as_ref().unwrap().success() {
        return Err(BuildError::Workspace {
            status: status.unwrap().code().unwrap(),
        });
    }

    Ok(())
}

fn build_wasm(guest: &Guest) -> Result<(), BuildError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&[
            "build",
            "--target=wasm32-wasi",
            "--release",
            &format!("--package={}", guest.name_output),
        ])
        .status();

    if !status.as_ref().unwrap().success() {
        let guest = guest.clone();
        return Err(BuildError::Wasm {
            guest_name: guest.name,
            guest_path: guest.path,
            status: status.unwrap().code().unwrap(),
        });
    }

    Ok(())
}

fn build_component(guest: &Guest) -> Result<(), BuildError> {
    let wasm_tools = env::var("WASM_TOOLS").unwrap_or_else(|_| "wasm-tools".to_string());

    let path_wasm_guest =
        dir_target().join(format!("wasm32-wasi/release/{}.wasm", guest.name_output));
    if !path_wasm_guest.exists() {
        let guest = guest.clone();
        return Err(BuildError::GuestFileNonExistent {
            guest_name: guest.name,
            guest_path: guest.path,
        });
    }

    let path_wasi_snapshot = project_root().join("wasi_snapshot_preview1.reactor.wasm.dev");
    if !path_wasi_snapshot.exists() {
        let guest = guest.clone();
        return Err(BuildError::WasiSnapshotFileNonExistent {
            guest_name: guest.name,
            guest_path: guest.path,
        });
    }

    if !&dir_components().exists() {
        let guest = guest.clone();
        return Err(BuildError::DirComponentsNonExistent {
            dir: dir_components(),
            guest_name: guest.name,
        });
    }

    let cmd_wasm_guest = path_wasm_guest.display().to_string();
    let cmd_wasi_snapshot = format!("--adapt={}", path_wasi_snapshot.display().to_string());
    let cmd_component_output = format!(
        "-o={}/{}-component.wasm",
        &dir_components().display(),
        guest.name
    );

    let status = Command::new(wasm_tools)
        .current_dir(project_root())
        .args(&[
            "component",
            "new",
            &cmd_wasm_guest,
            &cmd_wasi_snapshot,
            &cmd_component_output,
        ])
        .status();

    if !status.as_ref().unwrap().success() {
        let guest = guest.clone();
        return Err(BuildError::CommandFailed {
            guest_name: guest.name,
            path_wasm_guest: path_wasm_guest,
            path_wasi_snapshot: path_wasi_snapshot,
            dir_components: dir_components(),
            guest_path: guest.path,
            status: status.unwrap().code().unwrap(),
        });
    }

    Ok(())
}

async fn release() -> Result<(), ReleaseError> {
    if let Err(e) = try_release().await {
        return Err(e);
    }

    return Ok(());
}

async fn try_release() -> Result<(), ReleaseError> {
    let version_current: &str = env!("CARGO_PKG_VERSION");
    println!("🪵 [main.rs:239]~ version_current = {}", version_current);
    let version_input = env::args().nth(2);
    if version_input.is_none() {
        return Err(ReleaseError::MissingVersion);
    }

    if compare_to(version_current, version_input.clone().unwrap(), Cmp::Gt).unwrap() {
        return Err(ReleaseError::VersionLowerThanCurrent {
            version_input: version_input.unwrap(),
            version_current: version_current.to_string(),
        });
    }

    build_workspace_release().await?;
    rustwrap(version_arg).await?;

    return Ok(());
}

async fn build_workspace_release() -> Result<(), ReleaseError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--workspace", "--release"])
        .status();

    if status.is_err() {
        return Err(ReleaseError::BuildWorkspace {
            status: status.unwrap().code().unwrap(),
        });
    }

    return Ok(());
}

fn rustwrap(version: &str) -> Result<(), ReleaseError> {
    let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());

    let status = Command::new(rustwrap)
        .current_dir(project_root())
        .args(&[format("--tag {}", version)])
        .status()?;

    if !status.success() {
        return Err(format!("`rustwrap --tag {}", version_input));
    }

    return Ok(());
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn dir_target() -> PathBuf {
    project_root().join("target")
}

fn dir_components() -> PathBuf {
    project_root().join("components")
}

fn dir_guests() -> PathBuf {
    project_root().join("guests")
}
