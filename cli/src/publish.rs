#[allow(clippy::all)]
pub mod publish {
    use crate::paths::paths;

    use dialoguer::{theme::ColorfulTheme, Select};
    use log::{error, info};
    use octocrab::{self, models::repos::Release};
    use semver::Version;
    use std::{
        fs::{self},
        // io::Write,
        path::{Path, PathBuf},
        process::Command,
    };
    use thiserror::Error;

    // FIXME: impl custom Display trait on external Release type
    #[derive(Debug)]
    struct GitHubRelease {
        // id: ReleaseId,
        tag_name: String,
        // created_at: Option<DateTime<Utc>>,
    }

    impl ToString for GitHubRelease {
        fn to_string(&self) -> String {
            return self.tag_name.to_string();
        }
    }

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
        // TODO: remove "v" .replace()
        let version =
            Version::parse(&release.tag_name.replace("v", "")).expect("Could not parse release");

        create_dirs(version.clone())?;
        // TODO: this should be moved to release and binaries should be fetches from release.
        build_targets()?;
        copy_targets(version.clone())?;
        copy_npm(version.clone())?;

        // create_package(&release)?;
        // patch_package(&release)?;
        // publish_package(&release)?;

        Ok(())
    }

    fn create_dirs(version: Version) -> Result<(), PublishError> {
        fs::create_dir_all(paths::dir_npm_dist(version)).expect("Could not create directory");

        Ok(())
    }

    fn build_targets() -> Result<(), PublishError> {
        let targets = [
            "x86_64-unknown-linux-gnu", // # TODO: linux target support
            "x86_64-pc-windows-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ];

        for target in &targets {
            cross_build(target)?;
        }

        Ok(())
    }

    fn cross_build(target: &str) -> Result<(), PublishError> {
        let cross_build_cmd = Command::new("cross")
            .args(["build", "--release", "--target", target])
            .status()
            .expect("`cross build` Command failed");

        let target = target.to_string();

        return match cross_build_cmd.code() {
            Some(0) => Ok(()),
            Some(status) => Err(PublishError::BuildTarget { target, status }),
            None => Ok(()),
        };
    }

    fn copy_targets(version: Version) -> Result<(), PublishError> {
        let targets = [
            "x86_64-unknown-linux-gnu", // # TODO: linux target support
            "x86_64-pc-windows-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ];

        for target in targets {
            let src = paths::dir_target().to_owned().join(target);
            let src = src.as_path();
            let dest = paths::dir_npm_dist(version.clone()).to_owned().join(target);
            let dest = dest.as_path();

            copy_dir_to(src, dest).expect("Could not copy directory");
        }

        Ok(())
    }

    fn copy_npm(version: Version) -> Result<(), PublishError> {
        copy_dir_to(&paths::dir_npm(), &paths::dir_npm_dist(version))?;

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

    fn release_selection(selections: Vec<Release>) -> Result<usize, PublishError> {
        let selections: Vec<_> = selections
            .into_iter()
            .map(|release| {
                return GitHubRelease {
                    // id: release.id,
                    tag_name: release.tag_name,
                    // created_at: release.created_at,
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

    // fn create_package(release: &Release) -> Result<(), PublishError> {
    //     let rustwrap = env::var("RUSTWRAP").unwrap_or_else(|_| "rustwrap".to_string());
    //     let version =
    //         Version::parse(&release.tag_name).expect("Could not cast Release tag_name to Version");
    //     info!("Creating package for {version}");

    //     let rustwrap_cmd = Command::new(rustwrap)
    //         .current_dir(paths::dir_project_root())
    //         .args(["--tag", &release.tag_name.as_str()])
    //         .status()
    //         .expect("Failed to create package");

    //     return match rustwrap_cmd.code() {
    //         Some(0) => Ok(()),
    //         Some(status) => Err(PublishError::Rustwrap { version, status }),
    //         None => Ok(()),
    //     };
    // }

    fn copy_dir_to(src: &Path, dest: &Path) -> Result<(), PublishError> {
        if !src.is_dir() {
            return Err(PublishError::SourceNonExistent {
                src: src.to_path_buf(),
            });
        }

        if !dest.exists() {
            if let Err(error) = fs::create_dir_all(&dest) {
                return Err(PublishError::DestinationCreation {
                    dest: dest.to_path_buf(),
                    error,
                });
            }
        }

        let read_dir = match src.read_dir() {
            Ok(read_dir) => read_dir,
            Err(error) => {
                return Err(PublishError::ReadDir {
                    src: src.to_path_buf(),
                    error,
                })
            }
        };

        for entry in read_dir {
            let entry = match entry {
                Ok(entry_result) => entry_result,
                Err(error) => return Err(PublishError::ReadDirEntry { error }),
            };

            let entry_path = entry.path();
            let dest_child = dest.join(entry.file_name());

            if entry_path.is_dir() {
                copy_dir_to(&entry_path, &dest_child)?;
            } else {
                if let Err(error) = fs::copy(&entry_path, &dest_child) {
                    return Err(PublishError::CopyFile {
                        entry_path: entry_path,
                        dest_child: dest_child,
                        error,
                    });
                }
            }
        }

        Ok(())
    }

    // fn publish_package(release: &Release) -> Result<(), PublishError> {
    //     let npm = env::var("NPM").unwrap_or_else(|_| "npm".to_string());
    //     let tag_name = &release.tag_name;
    //     let tag = &release.tag_name;
    //     let version = Version::parse(&tag).expect("Could not cast Release tag_name to Version");
    //     info!("Publishing {version}");

    //     let npm_publish_cmd = Command::new(npm)
    //         .current_dir(paths::dir_npm_dist(&tag_name))
    //         .args(["publish"])
    //         .status()
    //         .expect("Failed to publish package");

    //     return match npm_publish_cmd.code() {
    //         Some(0) => {
    //             info!("Published {version}");
    //             return Ok(());
    //         }
    //         Some(status) => Err(PublishError::NpmPublish { version, status }),
    //         None => {
    //             info!("Published {version}");
    //             return Ok(());
    //         }
    //     };
    // }

    // pub fn replace_all_in_file(path: PathBuf, from: &str, to: &str) {
    //     let contents = read_to_string(path.clone()).expect("Could not read file: {path?}");
    //     let new = contents.replace(from, to);

    //     let mut file = OpenOptions::new()
    //         .write(true)
    //         .truncate(true)
    //         .open(path)
    //         .expect("Could not open file: {path}");

    //     file.write(new.as_bytes())
    //         .expect("Could not write file: {path}");
    // }

    #[derive(Debug, Error)]
    pub enum PublishError {
        #[error("Could not copy this non-existent source: {src}")]
        SourceNonExistent { src: PathBuf },
        #[error("Could not create destination `{dest}`. Error: {error}")]
        DestinationCreation {
            dest: PathBuf,
            error: std::io::Error,
        },
        #[error("Could not copy file {entry_path} to {dest_child}. Error: {error}")]
        CopyFile {
            entry_path: PathBuf,
            dest_child: PathBuf,
            error: std::io::Error,
        },
        #[error("Could not read dir {src}. Error: {error}")]
        ReadDir { src: PathBuf, error: std::io::Error },
        #[error("Could not read dir entry. Error: {error}")]
        ReadDirEntry { error: std::io::Error },
        #[error("Could not build {target}. Status code: {status}")]
        BuildTarget { target: String, status: i32 },
        #[error("There was an issue selecting release Page. Cause: {cause:#?}")]
        ReleasePageSelection { cause: octocrab::Error },
        #[error("Did not select a release")]
        DidNotSelectRelease,
        #[error("`npm publish` for {version} failed. Status code: {status}")]
        NpmPublish { version: Version, status: i32 },
    }
}
