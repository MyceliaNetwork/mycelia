pub mod release {
    use cargo_metadata::MetadataCommand;
    use log::error;
    use semver::Version;
    use std::{
        env,
        path::{Path, PathBuf},
        process::Command,
    };
    use thiserror::Error;

    pub async fn release() -> Result<(), ReleaseError> {
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
                // TODO: uncomment on merge
                // "--allow-branch",
                // "releases/*",
                // TODO: delete on merge
                "--no-git-commit",
            ])
            .status()
            .expect("Failed to bump workspaces version");

        return match cargo_ws_version.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::CargoWorkspace { status }),
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

    fn project_root() -> PathBuf {
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .unwrap()
            .to_path_buf()
    }

    #[derive(Debug, Error)]
    pub enum ReleaseError {
        #[error("cargo-workspace failed. Status code: {status}")]
        CargoWorkspace { status: i32 },
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
}
