use log::error;
use std::{
    cmp::Ordering,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use version_compare::{compare_to, Cmp};

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

    println!("{:#?}", env::args());

    match task.as_deref() {
        Some("build") => build()?,
        Some("release") => release().await?,
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
        build_workspace()?;
    }

    Ok(())
}

fn build_workspace() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--workspace"])
        .status()?;

    if !status.success() {
        Err(format!(
            "`cargo build --workspace` failed.

Status code: {}",
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

fn build_wasm(guest: &Guest) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&[
            "build",
            "--target=wasm32-wasi",
            "--release",
            &format!("--package={}", guest.name_output),
        ])
        .status()?;

    if !status.success() {
        Err(format!(
            "Build wasm '{}' failed.

Command: `cargo build --target wasm32-wasi --release --package {}`
Guest path: '{}'
Status code: {}",
            guest.name,
            guest.name,
            guest.path.display(),
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

fn build_component(guest: &Guest) -> Result<(), DynError> {
    let wasm_tools = env::var("WASM_TOOLS").unwrap_or_else(|_| "wasm-tools".to_string());

    let path_wasm_guest =
        dir_target().join(format!("wasm32-wasi/release/{}.wasm", guest.name_output));
    if !path_wasm_guest.exists() {
        Err(format!(
            "wasm guest file '{}' for '{}' does not exist",
            path_wasm_guest.display(),
            guest.name
        ))?;
    }

    let path_wasi_snapshot = project_root().join("wasi_snapshot_preview1.reactor.wasm.dev");
    if !path_wasi_snapshot.exists() {
        Err(format!(
            "wasi snapshot file '{}' for '{}' does not exist",
            path_wasi_snapshot.display(),
            guest.name
        ))?;
    }

    if !&dir_components().exists() {
        Err(format!(
            "component output directory '{}' for '{}' does not exist",
            &dir_components().display(),
            guest.name
        ))?;
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
        .status()?;

    if !status.success() {
        Err(format!(
            "Build component '{}' failed.

Command: `wasm-tools component new {} --adapt {} -o {}`
Guest path: '{}'
Status code: {}",
            guest.name,
            path_wasm_guest.display(),
            path_wasi_snapshot.display(),
            &dir_components().display(),
            guest.path.display(),
            status.code().unwrap()
        ))?;
    }

    Ok(())
}

async fn release() -> Result<(), DynError> {
    if let Err(e) = try_release().await {
        error!("{}", e);

        std::process::exit(-1);
    }
    return Ok(());
}

async fn try_release() -> Result<(), DynError> {
    let version_current: &str = env!("CARGO_PKG_VERSION");
    println!(
        "🪵 [main.rs:239]~ token ~ \x1b[0;32mversion_current\x1b[0m = {}",
        version_current
    );
    let version_arg = env::args().nth(2).expect("Version argument is missing");

    if compare_to(version_current, version_arg.clone(), Cmp::Gt).unwrap() {
        return Err(format!(
            "Version argument '{}' is lower than current version '{}'",
            version_arg, version_current
        ))?;
    }

    build_workspace_release().await?;
    // rustwrap(version_arg).await?;

    return Ok(());
}

async fn build_workspace_release() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--workspace", "--release"])
        .status()?;

    if !status.success() {
        // format!("`cargo build --workspace --release` failed.")
        return Err("x".to_string().into());
    }

    return Ok(());
}

// fn rustwrap(version: &str) -> Result<(), DynError> {
//     let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());

//     let status = Command::new(rustwrap)
//         .current_dir(project_root())
//         .args(&[format("--tag {}", version)])
//         .status()?;

//     if !status.success() {
//         return Err(format!("`rustwrap --tag {}", version_input));
//     }

//     return Ok(());
// }

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
