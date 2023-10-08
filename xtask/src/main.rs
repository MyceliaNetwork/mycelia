use log::{error, info};

use std::{
    cmp::Ordering,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;
use version_compare::{compare, Cmp};

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
    #[error("missing --version argument. Example: `--version 1.2.3`")]
    MissingVersionArg,
    #[error("missing --version value. Example: `--version 1.2.3`")]
    MissingVersionVal,
    #[error(
        "argument `--version {version_input:?}` is lower than the current version {version_current:?}"
    )]
    VersionLowerThanCurrent {
        version_input: String,
        version_current: String,
    },
    #[error(
        "argument `--version {version_input:?}` equal to the current version {version_current:?}"
    )]
    VersionEqualToCurrent {
        version_input: String,
        version_current: String,
    },
    #[error("Version comparison error")]
    VersionComparisonError,
    #[error(
        "building workspace failded.

Status code: {status:?}"
    )]
    BuildWorkspace { status: i32 },
    #[error(
        "`git branch releases/{version:?}` failed.

Status code: {status:?}"
    )]
    GitCreateBranch { version: String, status: i32 },
    #[error(
        "`git switch releases/{version:?}` failed.

Status code: {status:?}"
    )]
    GitSwitchBranch { status: i32, version: String },
    #[error(
        "`git add .` failed.

Status code: {status:?}"
    )]
    GitAddAll { status: i32 },
    #[error(
        "`commit -m \"Release {version:?}\"` failed.

Status code: {status:?}"
    )]
    GitCommit { version: String, status: i32 },
    #[error(
        "`git push origin -u releases/{version:?}` failed.

Status code: {status:?}"
    )]
    GitPushBranch { version: String, status: i32 },
    #[error(
        "`gh pr create --fill --label release --assignee @me --title \"Release {}\"` failed.

Status code: {status:?}"
    )]
    GitHubCreatePullRequest { version: String, status: i32 },
    #[error(
        "`rustwrap --tag {version:?}` failed.

Status code: {status:?}"
    )]
    Rustwrap { version: String, status: i32 },
}

type DynError = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() {
    if let Err(error) = try_main().await {
        error!("{error:#}");
        std::process::exit(-1);
    }
}

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);

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
    info!(
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
            status: status.unwrap().code().unwrap_or(-1),
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
            status: status.unwrap().code().unwrap_or(-1),
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
            status: status.unwrap().code().unwrap_or(-1),
        });
    }

    Ok(())
}

async fn release() -> Result<(), ReleaseError> {
    if let Err(error) = try_release().await {
        eprintln!("{error:#}");
        std::process::exit(-1);
    }
    return Ok(());
}

async fn try_release() -> Result<(), ReleaseError> {
    let version_current: &str = env!("CARGO_PKG_VERSION");
    let version_arg_tag = env::args().nth(2);
    let version_arg_val = env::args().nth(3);
    match version_arg_tag.clone() {
        None => return Err(ReleaseError::MissingVersionArg),
        Some(tag) => {
            if tag != "--version" {
                return Err(ReleaseError::MissingVersionArg);
            }
        }
    }
    if version_arg_val.clone().is_none() {
        return Err(ReleaseError::MissingVersionVal);
    }

    let version_comparison = compare(version_arg_val.clone().unwrap(), version_current);
    if version_comparison.is_err() {
        return Err(ReleaseError::VersionComparisonError);
    }
    match version_comparison.unwrap() {
        Cmp::Lt => {
            return Err(ReleaseError::VersionLowerThanCurrent {
                version_input: version_arg_val.clone().unwrap(),
                version_current: version_current.to_string(),
            });
        }
        Cmp::Eq => {
            return Err(ReleaseError::VersionEqualToCurrent {
                version_input: version_arg_val.clone().unwrap(),
                version_current: version_current.to_string(),
            });
        }
        _ => {}
    }

    build_workspace_release().await?;
    rustwrap(version_arg_val.unwrap()).await?;

    return Ok(());
}

async fn build_workspace_release() -> Result<(), ReleaseError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cargo_build = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--workspace", "--release"])
        .status();

    if cargo_build.is_err() {
        return Err(ReleaseError::BuildWorkspace {
            status: cargo_build.unwrap().code().unwrap_or(-1),
        });
    }

    return Ok(());
}
// ReleaseError::GitCreateBranch
// ReleaseError::GitSwitchBranch
// ReleaseError::GitAddAll
// ReleaseError::GitCommit
// ReleaseError::GitPushBranch

async fn git_create_branch(version: String) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let create_branch = Command::new(git)
        .current_dir(project_root())
        .args(&["branch", format!("releases/{}", version).as_str()])
        .status();

    if create_branch.is_err() {
        return Err(ReleaseError::GitCreateBranch {
            version,
            status: create_branch.unwrap().code().unwrap_or(-1),
        });
    }

    return Ok(());
}

async fn git_switch_branch(version: String) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let switch_branch = Command::new(git)
        .current_dir(project_root())
        .args(&["switch", format!("releases/{}", version).as_str()])
        .status();

    if switch_branch.is_err() {
        return Err(ReleaseError::GitSwitchBranch {
            version,
            status: switch_branch.unwrap().code().unwrap_or(-1),
        });
    }

    return Ok(());
}

async fn git_add_all() -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let add_all = Command::new(git)
        .current_dir(project_root())
        .args(&["add", "."])
        .status();

    if add_all.is_err() {
        return Err(ReleaseError::GitAddAll {
            status: add_all.unwrap().code().unwrap_or(-1),
        });
    }

    return Ok(());
}

async fn git_commit(version: String) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let commit = Command::new(git)
        .current_dir(project_root())
        .args(&["commit", "-m", format!("Release {}", version).as_str()])
        .status();

    if commit.is_err() {
        return Err(ReleaseError::GitCommit {
            status: commit.unwrap().code().unwrap_or(-1),
            version,
        });
    }

    return Ok(());
}

async fn git_push_branch(version: String) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let push_branch = Command::new(git)
        .current_dir(project_root())
        .args(&[
            "push",
            "-u",
            "origin",
            format!("releases/{}", version).as_str(),
        ])
        .status();

    if push_branch.is_err() {
        return Err(ReleaseError::GitPushBranch {
            status: push_branch.unwrap().code().unwrap_or(-1),
            version,
        });
    }

    return Ok(());
}

async fn github_create_pr(version: String) -> Result<(), ReleaseError> {
    let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
    let create_pr = Command::new(git)
        .current_dir(project_root())
        .args(&[
            "pr",
            "create",
            "--fill",
            "--label",
            "release",
            "--assignee",
            "@me",
            "--title",
            format!("Release {}", version).as_str(),
        ])
        .status();

    if create_pr.is_err() {
        return Err(ReleaseError::GitHubCreatePullRequest {
            status: push_branch.unwrap().code().unwrap_or(-1),
            version,
        });
    }

    return Ok(());
}

async fn rustwrap(version: String) -> Result<(), ReleaseError> {
    let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());

    let status = Command::new(rustwrap)
        .current_dir(project_root())
        .args(&[format!("--tag"), version.clone()])
        .status();

    if !status.as_ref().unwrap().success() {
        return Err(ReleaseError::Rustwrap {
            version,
            status: status.unwrap().code().unwrap_or(-1),
        });
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
