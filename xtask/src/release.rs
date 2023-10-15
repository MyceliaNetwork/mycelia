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
        bump()?;

        let tag = parse_cargo_pkg_tag();

        git_create_branch(tag.clone())?;
        git_switch_branch(tag.clone(), false)?;
        git_add_all(tag.clone())?;
        git_commit(tag.clone())?;
        git_push_branch(tag.clone())?;
        github_pr_create(tag.clone())?;
        github_release_create(tag.clone())?;

        Ok(())
    }

    // HACK: cargo's fill_env is called upon build, but after cargo-workspaces
    // updates the tag this is not reflected in the env variable.
    fn parse_cargo_pkg_tag() -> Version {
        let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let meta = MetadataCommand::new()
            .manifest_path("./Cargo.toml")
            .current_dir(&path)
            .exec()
            .unwrap();

        let root = meta.root_package().unwrap();
        let tag = &root.version;
        return tag.clone();
    }

    // TODO: replace DynError
    fn bump() -> Result<(), ReleaseError> {
        // cargo workspaces tag --allow-branch dani_cargo_run_new_cmd --no-git-push
        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let cargo_ws_tag = Command::new(cargo)
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
            .expect("Failed to bump workspaces tag");

        return match cargo_ws_tag.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::CargoWorkspace { status }),
            None => Ok(()),
        };
    }

    fn git_create_branch(tag: Version) -> Result<(), ReleaseError> {
        let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
        let create_branch = Command::new(git)
            .current_dir(project_root())
            .args(["branch", format!("releases/{}", tag).as_str()])
            .status()
            .expect("Failed to create git branch");

        return match create_branch.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::GitCreateBranch { tag, status }),
            None => Ok(()),
        };
    }

    fn git_switch_branch(tag: Version, switch_back: bool) -> Result<(), ReleaseError> {
        let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
        let branch = match switch_back {
            true => "-".to_string(),
            false => format!("releases/{}", tag.to_string()),
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
                    let tag = tag.clone();
                    git_switch_branch(tag, true)?;
                }
                return Err(ReleaseError::GitSwitchBranch {
                    tag: tag.clone(),
                    status: branch_already_exists,
                });
            }
            None => Ok(()),
        };
    }

    fn git_add_all(tag: Version) -> Result<(), ReleaseError> {
        let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
        let add_all = Command::new(git)
            .current_dir(project_root())
            .args(&["add", "."])
            .status()
            .expect("`git add .` failed");

        return match add_all.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::GitAddAll { tag, status }),
            None => Ok(()),
        };
    }

    fn git_commit(tag: Version) -> Result<(), ReleaseError> {
        let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
        let commit_msg = format!("Release {}", tag);
        let commit = Command::new(git)
            .current_dir(project_root())
            .args(&["commit", "-m", commit_msg.as_str()])
            .status()
            .expect("`git commit -m {commit_msg}");

        return match commit.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::GitCommit { tag, status }),
            None => Ok(()),
        };
    }

    fn git_push_branch(tag: Version) -> Result<(), ReleaseError> {
        let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
        let push_branch = Command::new(git)
            .current_dir(project_root())
            .args(["push", "-u", "origin", format!("releases/{}", tag).as_str()])
            .status()
            .expect("Failed to push git branch");

        return match push_branch.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::GitPushBranch { status, tag }),
            None => Ok(()),
        };
    }

    fn github_pr_create(tag: Version) -> Result<(), ReleaseError> {
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
                format!("releases/{}", tag.to_string()).as_str(),
                "--title",
                format!("Release {}", tag.to_string()).as_str(),
            ])
            .status()
            .expect("Failed to create GitHub pull request");

        return match create_pr.code() {
            Some(0) => Ok(()),
            Some(status) => Err(ReleaseError::GitHubCreatePullRequest { status, tag }),
            None => Ok(()),
        };
    }

    fn github_release_create(tag: Version) -> Result<(), ReleaseError> {
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
            Some(status) => Err(ReleaseError::GitHubReleaseCreate { tag, status }),
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
        #[error("`git branch releases/{tag}` failed. Status code: {status}")]
        GitCreateBranch { tag: Version, status: i32 },
        #[error("`git switch releases/{tag}` failed. Status code: {status}")]
        GitSwitchBranch { tag: Version, status: i32 },
        #[error("`git add .` failed for tag {tag}. Status code: {status}")]
        GitAddAll { tag: Version, status: i32 },
        #[error("`commit -m \"Release {tag:}\"` failed. Status code: {status:}")]
        GitCommit { tag: Version, status: i32 },
        #[error("`git push origin -u releases/{tag}` failed. Status code: {status}")]
        GitPushBranch { tag: Version, status: i32 },
        #[error("`gh pr create --fill --base releases/{tag} --assignee @me --title \"Release {tag}\"` failed. Status code: {status}" )]
        GitHubCreatePullRequest { tag: Version, status: i32 },
        // TODO: update final command
        #[error("`gh release create --prerelease --generate-notes` failed. Status code: {status}")]
        GitHubReleaseCreate { tag: Version, status: i32 },
    }
}
