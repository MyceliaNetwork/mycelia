#[allow(clippy::all)]
pub mod new {
    use crate::paths::paths;
    use dialoguer::{theme::ColorfulTheme, Input};
    use log::{error, info, trace};
    use std::{env, fs, process::Stdio};
    use thiserror::Error;
    use tokio::{process::Command, sync::oneshot::channel};

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

        let _ = fs::create_dir_all(&paths::deployable_target());

        let app_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Your app name")
            .interact_text()
            .unwrap();

        if let Err(error) = scaffold_next(app_name).await {
            return Err(error);
        }

        return Ok(());
    }

    async fn scaffold_next(app_name: String) -> Result<(), NewProjectError> {
        let _ = fs::create_dir_all(&paths::deployable_target());
        let (send, recv) = channel::<()>();
        let cargo = env::var("NPX").unwrap_or_else(|_| "npx".to_string());
        let mut npx_cmd = Command::new(cargo)
            .current_dir(&paths::deployable_target())
            .args(&[
                "create-next-app@latest",
                app_name.as_str(),
                "--name=blabli",
                "--typescript",
                "--eslint",
                "--tailwind",
                "--src-dir",
                "--no-app",
                "--no-import-alias",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("npx create-next-app failed");

        tokio::spawn(async move { send.send(()) });
        tokio::select! {
            wait = npx_cmd.wait() => trace!("wait {wait:#?}"),
            recv = recv => {
                match recv {
                    Ok(_) => {
                        println!("OKAY");
                    }
                    Err(error) => {

                        return Err(NewProjectError::CreateNextAppFailed { cause: error.to_string()});
                    }
                }
            }
        }

        return Ok(());
    }

    #[derive(Debug, Error)]
    enum NewProjectError {
        // #[error("npm/npx not found")]
        // NpmNotFound,
        #[error("npx create-next-app failed. Cause: {cause:#?}")]
        CreateNextAppFailed { cause: String },
    }
}
