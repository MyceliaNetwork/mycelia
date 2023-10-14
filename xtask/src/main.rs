use chrono::{DateTime, Utc};
use dialoguer::{theme::ColorfulTheme, Select};
#[allow(clippy::all)]
use log::{error, info};
// use octocrab::models::repos::Release;
use octocrab::{self, models::repos::Release, models::ReleaseId, Error};
use semver::Version;
use std::{
    cmp::Ordering,
    collections::HashMap,
    env, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

use cargo_metadata::MetadataCommand;

#[derive(Debug, Error)]
enum BuildError {
    #[error("`cargo build --workspace` failed. Status code: {status}")]
    Workspace { status: i32 },
    #[error(
        "Build wasm '{guest_name}' failed.

Command: `cargo build --target wasm32-wasi --release --package {guest_name}`
Guest path: {guest_path}
Status code: {status}"
    )]
    Wasm {
        guest_name: String,
        guest_path: PathBuf,
        status: i32,
    },
    #[error("wasm guest file '{guest_name}' for '{guest_path}' does not exist")]
    GuestFileNonExistent {
        guest_name: String,
        guest_path: PathBuf,
    },
    #[error(
        "Build component '{guest_name}' failed.

Command: `wasm-tools component new {path_wasm_guest} --adapt {path_wasi_snapshot} -o {dir_components}`
Guest path: '{guest_path}'
Status code: {status}"
    )]
    CommandFailed {
        guest_name: String,
        path_wasm_guest: PathBuf,
        path_wasi_snapshot: PathBuf,
        dir_components: PathBuf,
        guest_path: PathBuf,
        status: i32,
    },

    #[error("wasi snapshot file '{guest_name}' for '{guest_path}' does not exist")]
    WasiSnapshotFileNonExistent {
        guest_name: String,
        guest_path: PathBuf,
    },

    #[error("component output directory '{dir}' for '{guest_name}' does not exist")]
    DirComponentsNonExistent { dir: PathBuf, guest_name: String },
}
#[derive(Debug, Error)]
enum ReleaseError {
    #[error("building workspace failded. Status code: {status}")]
    BuildWorkspace { status: i32 },
    #[error("`git branch releases/{version}` failed. Status code: {status}")]
    GitCreateBranch { version: Version, status: i32 },
    #[error("`git switch releases/{version}` failed. Status code: {status}")]
    GitSwitchBranch { version: Version, status: i32 },
    #[error("`git add .` failed for version {version}. Status code: {status}")]
    GitAddAll { version: Version, status: i32 },
    #[error("`commit -m \"Release {version:}\"` failed. Status code: {status:}")]
    GitCommit { version: Version, status: i32 },
    #[error("`git push origin -u releases/{version}` failed. Status code: {status}")]
    GitPushBranch { version: Version, status: i32 },
    #[error("`gh pr create --fill --base releases/{version} --assignee @me --title \"Release {version}\"` failed. Status code: {status}" )]
    GitHubCreatePullRequest { version: Version, status: i32 },
    // TODO: update final command
    #[error("`gh release create --prerelease --generate-notes` failed. Status code: {status}")]
    GitHubReleaseCreate { version: Version, status: i32 },
}

#[derive(Debug, Error)]
enum PublishError {
    #[error("There was an issue with Releases. Cause: {cause:#?}")]
    ReleasesError { cause: Error },
    #[error("Did not select a release")]
    DidNotSelectRelease,
    #[error("`rustwrap --tag {version}` failed. Status code: {status}")]
    Rustwrap { version: Version, status: i32 },
}

type DynError = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::init();

    if let Err(error) = try_main().await {
        error!("{error:#}");
        std::process::exit(-1);
    }
}

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);

    match task.as_deref() {
        Some("build") => build().await?,
        Some("release") => release().await?,
        Some("publish") => publish().await?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    info!(
        "Tasks:

build    Build all guests and components
release  Release a new version
publish  Publish a new version"
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
    let name_map = HashMap::from([("function", "mycelia_guest_function")]);
    let priority = vec!["*".to_string(), "function".to_string()];

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

        return match a.cmp(&b) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Greater,
        };
    });

    return guests_filtered;
}

async fn build() -> Result<(), DynError> {
    fs::create_dir_all(&dir_target())?;
    fs::create_dir_all(&dir_components())?;

    for guest in guests() {
        build_wasm(&guest)?;
        build_component(&guest)?;
        build_workspace()?;
    }

    Ok(())
}

fn build_workspace() -> Result<(), BuildError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(["build", "--workspace"])
        .status()
        .expect("Failed to build workspace");

    return match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(BuildError::Workspace { status: code }),
        None => Ok(()),
    };
}

fn build_wasm(guest: &Guest) -> Result<(), BuildError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let build_wasm = Command::new(cargo)
        .current_dir(project_root())
        .args([
            "build",
            "--target=wasm32-wasi",
            "--release",
            &format!("--package={}", guest.name_output),
        ])
        .status()
        .expect("Failed to build wasm guest");

    return match build_wasm.code() {
        Some(0) => Ok(()),
        Some(status) => {
            let guest = guest.clone();
            Err(BuildError::Wasm {
                guest_name: guest.name,
                guest_path: guest.path,
                status,
            })
        }
        None => Ok(()),
    };
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
    let cmd_wasi_snapshot = format!("--adapt={}", path_wasi_snapshot.display());
    let cmd_component_output = format!(
        "-o={}/{}-component.wasm",
        &dir_components().display(),
        guest.name
    );

    let wasm_tools = Command::new(wasm_tools)
        .current_dir(project_root())
        .args([
            "component",
            "new",
            &cmd_wasm_guest,
            &cmd_wasi_snapshot,
            &cmd_component_output,
        ])
        .status()
        .expect("Failed to build component with wasm_tools");

    return match wasm_tools.code() {
        Some(0) => Ok(()),
        Some(status) => {
            let guest = guest.clone();
            return Err(BuildError::CommandFailed {
                guest_name: guest.name,
                path_wasm_guest,
                path_wasi_snapshot,
                dir_components: dir_components(),
                guest_path: guest.path,
                status,
            });
        }
        None => Ok(()),
    };
}

async fn release() -> Result<(), ReleaseError> {
    if let Err(error) = try_release().await {
        error!("{error:#}");

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_release() -> Result<(), ReleaseError> {
    bump().await?;

    let version = parse_cargo_pkg_version();

    git_create_branch(version.clone()).await?;
    git_switch_branch(version.clone(), false)?;
    git_add_all(version.clone())?;
    git_commit(version.clone())?;
    git_push_branch(version.clone()).await?;
    github_pr_create(version.clone()).await?;
    github_release_create(version.clone()).await?;

    Ok(())
}

// HACK: cargo's fill_env is called upon build, but after cargo-workspaces
// updates the version this is not reflected in the env variable.
fn parse_cargo_pkg_version() -> Version {
    let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .current_dir(&path)
        .exec()
        .unwrap();

    let root = meta.root_package().unwrap();
    let version = &root.version;
    return version.clone();
}

// TODO: replace DynError
async fn bump() -> Result<(), ReleaseError> {
    // cargo workspaces version --allow-branch dani_cargo_run_new_cmd --no-git-push
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cargo_ws_version = Command::new(cargo)
        .current_dir(project_root())
        .args([
            "workspaces",
            "version",
            "--allow-branch",
            "releases/*",
            "--no-git-commit",
        ])
        .status()
        .expect("Failed to bump workspaces version");

    return match cargo_ws_version.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::BuildWorkspace { status }),
        None => Ok(()),
    };
}

fn build_workspace_release() -> Result<(), ReleaseError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cargo_build = Command::new(cargo)
        .current_dir(project_root())
        .args(["build", "--workspace", "--release"])
        .status()
        .expect("Failed to build workspace");

    return match cargo_build.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::BuildWorkspace { status }),
        None => Ok(()),
    };
}

async fn git_create_branch(version: Version) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let create_branch = Command::new(git)
        .current_dir(project_root())
        .args(["branch", format!("releases/{}", version).as_str()])
        .status()
        .expect("Failed to create git branch");

    return match create_branch.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitCreateBranch { version, status }),
        None => Ok(()),
    };
}

fn git_switch_branch(version: Version, switch_back: bool) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let branch = match switch_back {
        true => "-".to_string(),
        false => format!("releases/{}", version.to_string()),
    };
    let switch_branch = Command::new(git)
        .current_dir(project_root())
        .args(["switch", branch.as_str()])
        .status()
        .expect("Failed to switch git branch");

    return match switch_branch.code() {
        Some(0) => Ok(()),
        Some(status) => {
            let branch_already_exists: i32 = 128;
            if status == branch_already_exists {
                git_switch_branch(version.clone(), true)?;
            }
            return Err(ReleaseError::GitSwitchBranch {
                version,
                status: branch_already_exists,
            });
        }
        None => Ok(()),
    };
}

fn git_add_all(version: Version) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let add_all = Command::new(git)
        .current_dir(project_root())
        .args(&["add", "."])
        .status()
        .expect("`git add .` failed");

    return match add_all.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitAddAll { version, status }),
        None => Ok(()),
    };
}

fn git_commit(version: Version) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let commit_msg = format!("Release {}", version);
    let commit = Command::new(git)
        .current_dir(project_root())
        .args(&["commit", "-m", commit_msg.as_str()])
        .status()
        .expect("`git commit -m {commit_msg}");

    return match commit.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitCommit { version, status }),
        None => Ok(()),
    };
}

async fn git_push_branch(version: Version) -> Result<(), ReleaseError> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
    let push_branch = Command::new(git)
        .current_dir(project_root())
        .args([
            "push",
            "-u",
            "origin",
            format!("releases/{}", version).as_str(),
        ])
        .status()
        .expect("Failed to push git branch");

    return match push_branch.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitPushBranch { status, version }),
        None => Ok(()),
    };
}

async fn github_pr_create(version: Version) -> Result<(), ReleaseError> {
    let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
    let create_pr = Command::new(github)
        .current_dir(project_root())
        .args([
            "pr",
            "create",
            "--fill",
            "--assignee",
            "@me",
            "--base",
            format!("releases/{}", version.to_string()).as_str(),
            "--title",
            format!("Release {}", version.to_string()).as_str(),
            "--verify-tag",
        ])
        .status()
        .expect("Failed to create GitHub pull request");

    return match create_pr.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitHubCreatePullRequest { status, version }),
        None => Ok(()),
    };
}

async fn github_release_create(version: Version) -> Result<(), ReleaseError> {
    let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
    let create_release = Command::new(github)
        .current_dir(project_root())
        .args([
            "release",
            "create",
            "--prerelease", // TODO: remove this flag when we are ready for a stable release
            "--generate-notes",
        ])
        .status()
        .expect("Failed to create GitHub release");

    return match create_release.code() {
        Some(0) => Ok(()),
        Some(status) => Err(ReleaseError::GitHubReleaseCreate { version, status }),
        None => Ok(()),
    };
}

async fn publish() -> Result<(), PublishError> {
    if let Err(error) = try_publish().await {
        error!("{error:#}");

        std::process::exit(-1);
    }

    Ok(())
}

async fn try_publish() -> Result<(), PublishError> {
    info!("Publishing release");
    let releases = github_release_list().await;
    let releases = match releases {
        Ok(releases) => releases,
        Err(cause) => return Err(PublishError::ReleasesError { cause }),
    };
    let releases = releases.items;
    let selection = release_selection(releases.clone());
    let selection = selection.expect("Release selection error");
    let selection = &releases[selection];
    println!(
        "🪵 [main.rs:551]~ token ~ \x1b[0;32mselection\x1b[0m = {:#?}",
        selection
    );

    replace_all_in_file(file_rustwrap(), "__VERSION__", &selection.tag_name);
    // publish_pkg(selection);

    Ok(())
}

fn replace_all_in_file(path: PathBuf, from: &str, to: &str) {
    let contents = fs::read_to_string(path.clone()).expect("Could not read file: {path?}");
    let new = contents.replace(from, to);
    dbg!(&contents, &new);

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .expect("Could not open file: {path}");
    file.write(new.as_bytes())
        .expect("Could not write file: {path}");
}

async fn github_release_list() -> Result<octocrab::Page<Release>, octocrab::Error> {
    let octocrab = octocrab::instance();
    let page = octocrab
        .repos("MyceliaNetwork", "mycelia")
        .releases()
        .list()
        // Optional Parameters
        .per_page(100)
        // .page(5u32)
        // Send the request
        .send()
        .await;

    return page;
}

#[derive(Debug)]
struct MyceliaRelease {
    id: ReleaseId,
    tag_name: String,
    created_at: Option<DateTime<Utc>>,
}

impl ToString for MyceliaRelease {
    fn to_string(&self) -> String {
        let id = self.id;
        let date = self
            .created_at
            .expect("Release created at DateTime")
            .to_string();
        let name = &self.tag_name;
        let padding_name = 32;
        let padding_date = 25;
        let padding_id = 11;

        return format!("┌{name:─^padding_name$}┐ ┌{date:─^padding_date$}┐ ┌{id:─^padding_id$}┐ ");
    }
}

fn release_selection(selections: Vec<Release>) -> Result<usize, PublishError> {
    let selections: Vec<_> = selections
        .into_iter()
        .map(|release| {
            let r = MyceliaRelease {
                id: release.id,
                tag_name: release.tag_name,
                created_at: release.created_at,
            };
            return r;
        })
        .collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose which release you'd like to publish")
        .default(0)
        .items(&selections[..])
        .interact_opt()
        .expect("Release Selection failed");

    return match selection {
        Some(selection) => Ok(selection),
        None => Err(PublishError::DidNotSelectRelease),
    };
}

fn publish_pkg(release: Release) -> Result<(), PublishError> {
    let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());
    let version =
        Version::parse(&release.tag_name).expect("Could not cast Release tag_name to Version");
    let rustwrap = Command::new(rustwrap)
        .current_dir(project_root())
        .args(["--tag", version.to_string().as_str()])
        .status()
        .expect("Failed to publish package");

    return match rustwrap.code() {
        Some(0) => Ok(()),
        Some(status) => Err(PublishError::Rustwrap { version, status }),
        None => Ok(()),
    };
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

fn file_rustwrap() -> PathBuf {
    project_root().join("rustwrap.yaml")
}
