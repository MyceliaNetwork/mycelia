pub mod release {
    use log::error;

    type DynError = Box<dyn std::error::Error>;

    pub async fn release() -> Result<(), DynError> {
        if let Err(error) = try_release().await {
            error!("{error:#}");

            std::process::exit(-1);
        }

        Ok(())
    }

    async fn try_release() -> Result<(), DynError> {
        let branches = github::release_branches().await?;
        let branch_index = github::select_release(branches.clone().items).await?;
        let branch = &branches.items[branch_index];
        github::create_release(branch).await?;

        Ok::<(), DynError>(())
    }

    pub mod github {
        use dialoguer::{theme::ColorfulTheme, Select};
        use log::info;
        use octocrab::{self, models::repos::Branch, Error, Octocrab};
        use thiserror::Error;

        pub async fn release_branches() -> Result<octocrab::Page<Branch>, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            };

            let branches = octocrab::instance()
                .repos("MyceliaNetwork", "mycelia")
                .list_branches()
                .send()
                .await;

            return match branches {
                Ok(branches) => Ok(branches),
                Err(error) => return Err(GitHubError::ListBranches { error }),
            };
        }
        #[derive(Debug)]
        pub struct GitHubBranch {
            name: String,
        }

        impl ToString for GitHubBranch {
            fn to_string(&self) -> String {
                return format!("{}", self.name);
            }
        }

        pub async fn select_release(branches: Vec<Branch>) -> Result<usize, GitHubError> {
            let selections: Vec<_> = branches
                .into_iter()
                .filter(|branch: &Branch| {
                    let b = println!(
                        "🪵 [release.rs:62]~ token ~ \x1b[0;32mbranch.name\x1b[0m = {}",
                        branch.name
                    );
                    branch.name.starts_with("/release/")
                })
                .map(|branch: Branch| {
                    return GitHubBranch { name: branch.name };
                })
                .collect();

            println!(
                "🪵 [release.rs:52]~ token ~ \x1b[0;32mselections\x1b[0m = {:#?}",
                selections
            );

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose which RC you would like to release")
                .default(0)
                .items(&selections[..])
                .interact_opt()
                .expect("RC selection failed");

            return match selection {
                Some(selection) => Ok(selection),
                None => Err(GitHubError::DidNotSelectRc),
            };
        }

        pub async fn create_release(branch: &Branch) -> Result<(), GitHubError> {
            info!("Creating release for {}", branch.name);

            Ok(())
        }

        pub fn env_token() -> Result<String, GitHubError> {
            // TODO: Next step would be to hide the `release` feature for users that do not have the rights
            let token = std::env::var("GITHUB_TOKEN");
            return match token {
                Ok(token) => Ok(token),
                _ => Err(GitHubError::EnvTokenNotFound),
            };
        }

        #[derive(Debug, Error)]
        pub enum GitHubError {
            #[error("GITHUB_TOKEN environment variable not found. IMPORTANT: add it to the .gitignored /.env file in the project root to make sure your secrets do not leak.")]
            EnvTokenNotFound,
            #[error("Bad GitHub credentials. Please check if the GITHUB_TOKEN in your /.env file is correctly configured and has the required permissions.")]
            EnvTokenInvalid,
            #[error("`Octocrab::builder().personal_token(token).build()` failed. Error: {error}")]
            OctocrabTokenBuild { error: Error },
            #[error("Did not select a release")]
            DidNotSelectRelease,
            #[error("Did not select a RC")]
            DidNotSelectRc,
            #[error("Error getting branches. Error: {error}")]
            ListBranches { error: Error },
        }
    }
}
