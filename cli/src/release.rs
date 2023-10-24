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
        let branches = github::branches().await?;
        let branch_index = github::select_release(branches.clone()).await?;
        let branch_name = &branches[branch_index];
        github::create_release(branch_name).await?;

        Ok::<(), DynError>(())
    }

    pub mod github {
        use dialoguer::{theme::ColorfulTheme, Select};
        use log::info;
        use octocrab::{
            self,
            models::repos::{Branch, Release},
            Error,
        };
        use thiserror::Error;

        pub async fn release_branches() -> Result<Vec<String>, GitHubError> {
            let releases = octocrab::instance()
                .repos("MyceliaNetwork", "mycelia")
                .releases()
                .list()
                .send()
                .await;

            let releases: Vec<Release> = match releases {
                Ok(releases) => releases.items,
                Err(error) => return Err(GitHubError::ListReleases { error }),
            };

            let releases = releases
                .into_iter()
                .map(|release| release.target_commitish)
                .collect::<Vec<_>>();

            return Ok(releases);
        }

        pub async fn branches() -> Result<Vec<String>, GitHubError> {
            let token = env_token()?;
            // let octocrab = Octocrab::builder().personal_token(token).build();
            // let octocrab = match octocrab {
            //     Ok(octocrab) => octocrab,
            //     Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            // };

            let branches = octocrab::instance()
                .repos("MyceliaNetwork", "mycelia")
                .list_branches()
                .send()
                .await;

            return match branches {
                Ok(branches) => Ok(branches.into_iter().map(|branch| branch.name).collect()),
                Err(error) => return Err(GitHubError::ListBranches { error }),
            };
        }

        // FIXME: impl custom Display trait on external Branch type
        #[derive(Debug)]
        pub struct GitHubBranch {
            name: String,
        }

        impl ToString for GitHubBranch {
            fn to_string(&self) -> String {
                return format!("{}", self.name);
            }
        }

        pub async fn select_release(branches: Vec<String>) -> Result<usize, GitHubError> {
            let release_branches = release_branches().await?;
            let selections: Vec<_> = branches
                .into_iter()
                .filter(|branch_name| {
                    let unreleased = !release_branches.contains(branch_name);
                    unreleased && branch_name.starts_with("release/")
                })
                .collect();

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

        pub async fn create_release(branch_name: &String) -> Result<(), GitHubError> {
            info!("Creating release for {}", branch_name);

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
            #[error("Error getting releases. Error: {error}")]
            ListReleases { error: Error },
        }
    }
}
