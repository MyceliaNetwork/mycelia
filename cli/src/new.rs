#[allow(clippy::all)]
pub mod new {
    use crate::paths::paths;
    use log::{error, info};
    use std::{env, fs};
    use thiserror::Error;
    use tokio::process::Command;

    type DynError = Box<dyn std::error::Error>;

    // TODO: create backend/ directory
    pub async fn new() -> Result<(), DynError> {
        if let Err(error) = try_new().await {
            error!("{error:?}");

            std::process::exit(-1);
        }

        return Ok(());
    }

    async fn try_new() -> Result<(), NewProjectError> {
        info!("Creating new Mycelia project");

        let _ = fs::create_dir_all(&paths::dir_deployable_target());

        if let Err(error) = scaffold_next().await {
            return Err(error);
        }

        return Ok(());
    }

    async fn scaffold_next() -> Result<(), NewProjectError> {
        let _ = fs::create_dir_all(&paths::dir_deployable_target());
        let npx = env::var("NPX").unwrap_or_else(|_| "npx".to_string());
        let npx_cmd = Command::new(npx)
            .current_dir(&paths::dir_deployable_target())
            // Next.js app router is disabled as it relies on server components.
            // We technically cannot support this at the moment.
            .args(&["create-next-app@latest", "--no-app"])
            .status()
            .await
            .expect("Failed to run `npx create-next-app --no-app`");

        return match npx_cmd.code() {
            Some(0) => Ok(()),
            Some(1) => Err(NewProjectError::SetupTerminated),
            Some(2) => Err(NewProjectError::NpmNotFound),
            Some(code) => Err(NewProjectError::CreateNextAppFailed { code }),
            None => Ok(()),
        };
    }

    #[derive(Debug, Error)]
    enum NewProjectError {
        #[error("npm/npx not found. Please install npm at https://www.npmjs.com/ or through your OS' package manager. Then try again.")]
        NpmNotFound,
        #[error("Setup terminated.")]
        SetupTerminated,
        #[error("`npx create-next-app --no-app` failed. Code: {code:#?}")]
        CreateNextAppFailed { code: i32 },
    }
}
