#[allow(clippy::all)]
pub mod publish {
    use crate::paths::paths;
    use chrono::{DateTime, Utc};
    use dialoguer::{theme::ColorfulTheme, Select};
    use log::{error, info};
    use octocrab::{self, models::repos::Release, models::ReleaseId, Error};
    use semver::Version;
    use std::{
        env,
        fs::{read_to_string, OpenOptions},
        io::Write,
        path::PathBuf,
        process::Command,
    };
    use thiserror::Error;

    pub async fn publish() -> Result<(), PublishError> {
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
            Err(cause) => return Err(PublishError::ReleasePageSelection { cause }),
        };
        let releases = releases.items;
        let selection = release_selection(releases.clone());
        let selection = selection.expect("Release selection error");
        let release = &releases[selection];

        create_package(&release)?;
        patch_package(&release)?;
        publish_package(&release)?;

        Ok(())
    }

    async fn github_release_list() -> Result<octocrab::Page<Release>, octocrab::Error> {
        let octocrab = octocrab::instance();
        let page = octocrab
            .repos("MyceliaNetwork", "mycelia")
            .releases()
            .list()
            // Send the request
            .send()
            .await;

        return page;
    }

    // FIXME: impl custom Display trait on external Release type
    #[derive(Debug)]
    struct GitHubRelease {
        id: ReleaseId,
        tag_name: String,
        created_at: Option<DateTime<Utc>>,
    }

    impl ToString for GitHubRelease {
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

            return format!(
                "┌{name:─^padding_name$}┐ ┌{date:─^padding_date$}┐ ┌{id:─^padding_id$}┐ "
            );
        }
    }

    fn release_selection(selections: Vec<Release>) -> Result<usize, PublishError> {
        let selections: Vec<_> = selections
            .into_iter()
            .map(|release| {
                return GitHubRelease {
                    id: release.id,
                    tag_name: release.tag_name,
                    created_at: release.created_at,
                };
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

    fn create_package(release: &Release) -> Result<(), PublishError> {
        let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());
        let version = Version::parse(&release.tag_name.replace("v", ""))
            .expect("Could not cast Release tag_name to Version");
        info!("Creating package for {version}");

        let rustwrap_cmd = Command::new(rustwrap)
            .current_dir(paths::project_root())
            .args(["--tag", &release.tag_name.as_str()])
            .status()
            .expect("Failed to create package");

        return match rustwrap_cmd.code() {
            Some(0) => Ok(()),
            Some(status) => Err(PublishError::Rustwrap { version, status }),
            None => Ok(()),
        };
    }

    fn patch_package(release: &Release) -> Result<(), PublishError> {
        replace_all_in_file(
            paths::file_npm_package_manifest(&release.tag_name.clone()),
            "  \"name\": \"mycelia\",",
            "  \"name\": \"@departurelabs/mycelia\",",
        );

        let bins = vec![
            "mycelia-bin-win32-x64",
            "mycelia-bin-linux-x64",
            "mycelia-bin-darwin-x64",
            "mycelia-bin-darwin-arm64",
        ];

        for bin in bins.iter() {
            replace_all_in_file(paths::file_npm_info(&release.tag_name), bin, "mycelia/bin");
        }

        Ok(())
    }

    fn publish_package(release: &Release) -> Result<(), PublishError> {
        let npm = env::var("NPM").unwrap_or_else(|_| "npm".to_string());
        let tag_name = &release.tag_name;
        let tag = &release.tag_name.replace("v", "");
        let version = Version::parse(&tag).expect("Could not cast Release tag_name to Version");
        info!("Publishing {version}");

        let npm_publish_cmd = Command::new(npm)
            .current_dir(paths::dir_npm_package(&tag_name))
            .args(["publish"])
            .status()
            .expect("Failed to publish package");

        return match npm_publish_cmd.code() {
            Some(0) => {
                info!("Published {version}");
                return Ok(());
            }
            Some(status) => Err(PublishError::NpmPublish { version, status }),
            None => {
                info!("Published {version}");
                return Ok(());
            }
        };
    }

    pub fn replace_all_in_file(path: PathBuf, from: &str, to: &str) {
        let contents = read_to_string(path.clone()).expect("Could not read file: {path?}");
        let new = contents.replace(from, to);

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .expect("Could not open file: {path}");

        file.write(new.as_bytes())
            .expect("Could not write file: {path}");
    }

    #[derive(Debug, Error)]
    pub enum PublishError {
        #[error("There was an issue selecting release Page. Cause: {cause:#?}")]
        ReleasePageSelection { cause: Error },
        #[error("Did not select a release")]
        DidNotSelectRelease,
        #[error("`rustwrap --tag {version}` failed. Status code: {status}")]
        Rustwrap { version: Version, status: i32 },
        #[error("`npm publish` for {version} failed. Status code: {status}")]
        NpmPublish { version: Version, status: i32 },
    }
}
