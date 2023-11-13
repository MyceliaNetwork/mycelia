#[allow(clippy::all)]
pub mod new {
    use crate::paths::paths;
    use dialoguer::{theme::ColorfulTheme, Input};
    use log::{error, info};
    use std::{env, fs};
    use thiserror::Error;
    use tokio::process::Command;

    type DynError = Box<dyn std::error::Error>;

    pub async fn new() -> Result<(), DynError> {
        if let Err(error) = try_new().await {
            error!("{error:?}");

            std::process::exit(-1);
        }

        return Ok(());
    }

    async fn try_new() -> Result<(), NewProjectError> {
        info!("Creating new Mycelia project");

        let app_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Your app name")
            .interact_text()
            .unwrap();

        if let Err(error) = scaffold_next(app_name.clone()).await {
            return Err(error);
        }
        let _ = fs::create_dir_all(&paths::dir_deployable_backend(app_name));

        return Ok(());
    }

    async fn scaffold_next(app_name: String) -> Result<(), NewProjectError> {
        let _ = fs::create_dir_all(&paths::dir_deployable());
        let npx = env::var("NPX").unwrap_or_else(|_| "npx".to_string());
        let npx_cmd = Command::new(npx)
            .current_dir(&paths::dir_deployable())
            .args(&[
                "create-next-app@latest",
                app_name.as_str(),
                // Next.js app router is disabled as it relies on server components.
                // We technically cannot support this at the moment.
                "--no-app",
            ])
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
