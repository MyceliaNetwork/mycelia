#[allow(clippy::all)]
pub mod release {
    use crate::paths::paths;
    use log::error;
    use semver::Version;

    type DynError = Box<dyn std::error::Error>;

    pub enum Branch<'a> {
        Back(&'a Version),
        Tag(&'a Version),
    }

    pub async fn release() -> Result<(), DynError> {
        if let Err(error) = try_release().await {
            error!("{error:#}");

            std::process::exit(-1);
        }

        Ok(())
    }

    async fn try_release() -> Result<(), DynError> {
        github::env_token()?;
        git::status()?;

        workspace::bump()?;

        let tag = workspace::parse_cargo_pkg_version();

        workspace::replace_all_in_file(
            paths::file_rustwrap(),
            "__VERSION__",
            tag.to_string().as_str(),
        );

        git::create_branch(tag.clone())?;
        git::switch_branch(Branch::Tag(&tag))?;
        git::add_all(tag.clone())?;
        git::commit(tag.clone())?;
        git::push_branch(tag.clone())?;
        github::pr_create(tag.clone()).await?;
        // github::release_create(tag.clone())?;

        git::switch_branch(Branch::Back(&tag))?;

        Ok::<(), DynError>(())
    }

    mod workspace {
        use crate::paths::paths;
        use cargo_metadata::MetadataCommand;
        use semver::Version;
        use std::{env, fs, fs::OpenOptions, io::Write, path::PathBuf, process::Command};
        use thiserror::Error;

        // HACK: cargo's fill_env is called upon build, but after cargo-workspaces
        // updates the tag this is not reflected in the env variable.
        pub fn parse_cargo_pkg_version() -> Version {
            let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            let meta_cmd = MetadataCommand::new()
                .manifest_path("./Cargo.toml")
                .current_dir(&path)
                .exec()
                .expect("Failed to read CARGO_MANIFEST_DIR/Cargo.toml");

            let root = meta_cmd.root_package().unwrap();
            let tag = &root.version;
            return tag.clone();
        }

        pub fn bump() -> Result<(), WorkspaceError> {
            let pre_bump_tag = parse_cargo_pkg_version();
            let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
            let cargo_workspaces_version_cmd = Command::new(cargo)
                .current_dir(paths::project_root())
                .args([
                    "workspaces",
                    "version",
                    // TODO: uncomment on merge
                    // "--allow-branch",
                    // "release/*",
                    "--no-git-commit",
                ])
                .status()
                .expect("`cargo workspaces version --no-git-commit` failed");

            let post_bump_tag = parse_cargo_pkg_version();

            if pre_bump_tag == post_bump_tag {
                return Err(WorkspaceError::DidNotUpdateVersion { tag: post_bump_tag });
            }

            return match cargo_workspaces_version_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(WorkspaceError::CargoWorkspace { status }),
                None => Ok(()),
            };
        }

        pub fn replace_all_in_file(path: PathBuf, from: &str, to: &str) {
            let contents = fs::read_to_string(path.clone()).expect("Could not read file: {path?}");
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
        pub enum WorkspaceError {
            #[error("Chosen tag is same as current ({tag}). You might have not confirmed.")]
            DidNotUpdateVersion { tag: Version },
            #[error("cargo-workspace failed. Status code: {status}")]
            CargoWorkspace { status: i32 },
        }
    }

    pub mod git {
        use crate::paths::paths;
        use crate::release::release::github;
        use crate::release::release::Branch;
        use semver::Version;
        use std::{env, process::Command};
        use thiserror::Error;

        pub fn status() -> Result<(), GitError> {
            let git: String = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let status_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["status"])
                .output()
                .expect("`git status` failed")
                .stdout;

            let output = String::from_utf8(status_cmd).expect("Failed to convert status to utf-8");

            return match output.contains("nothing to commit, working tree clean") {
                true => Ok(()),
                false => Err(GitError::Status { output }),
            };
        }

        pub fn create_branch(tag: Version) -> Result<(), GitError> {
            let git: String = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let username = github::get_username();
            let branch_name = format!("rc/{username}_{tag}");
            let create_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["branch", branch_name.as_str()])
                .status()
                .expect(format!("`git branch {branch_name}` failed").as_str());

            return match create_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(GitError::CreateBranch { tag, status }),
                None => Ok(()),
            };
        }

        pub fn switch_branch(branch: Branch) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let username = github::get_username();
            let branch_arg = match branch {
                Branch::Back(_) => "-".to_string(),
                Branch::Tag(tag) => format!("rc/{username}_{tag}").to_string(),
            };
            let tag = match branch {
                Branch::Back(tag) => tag,
                Branch::Tag(tag) => tag,
            };
            let git_switch_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["switch", branch_arg.as_str()])
                .status()
                .expect(format!("Failed to run `git switch {branch_arg}").as_str());

            return match git_switch_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => {
                    let branch_already_exists: i32 = 128;
                    if status == branch_already_exists {
                        switch_branch(Branch::Back(tag))?;
                    }
                    return Err(GitError::SwitchBranch {
                        tag: tag.clone(),
                        status: branch_already_exists,
                    });
                }
                None => Ok(()),
            };
        }

        pub fn add_all(tag: Version) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let git_add_all_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(&["add", "."])
                .status()
                .expect("Failed to run `git add .`");

            return match git_add_all_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(GitError::AddAll { tag, status }),
                None => Ok(()),
            };
        }

        pub fn commit(tag: Version) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let commit_msg = format!("Release {tag}");
            let git_commit_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(&["commit", "-m", commit_msg.as_str()])
                .status()
                .expect(format!("failed to run `git commit -m {commit_msg}").as_str());

            return match git_commit_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(GitError::Commit { tag, status }),
                None => Ok(()),
            };
        }

        pub fn push_branch(tag: Version) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let username = github::get_username();
            let branch_name = format!("rc/{username}_{tag}").to_string();
            let branch_name = branch_name.as_str();
            let git_push_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["push", "-u", "origin", branch_name])
                .status()
                .expect(format!("Failed to run `git push -u origin {branch_name}").as_str());

            return match git_push_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => {
                    let branch_already_exists: i32 = 128;
                    if status == branch_already_exists {
                        switch_branch(Branch::Back(&tag))?;
                    }
                    return Err(GitError::PushBranch {
                        tag: tag.clone(),
                        status: branch_already_exists,
                    });
                }
                None => Ok(()),
            };
        }

        #[derive(Debug, Error)]
        pub enum GitError {
            #[error(
                "`git status` failed. Please commit your changes first. Output:

{output:#?}"
            )]
            Status { output: String },
            #[error("`git branch rc/{tag}` failed. Status code: {status}")]
            CreateBranch { tag: Version, status: i32 },
            #[error("`git switch rc/{tag}` failed. Status code: {status}")]
            SwitchBranch { tag: Version, status: i32 },
            #[error("`git add .` failed for tag {tag}. Status code: {status}")]
            AddAll { tag: Version, status: i32 },
            #[error("`commit -m \"Release {tag:}\"` failed. Status code: {status:}")]
            Commit { tag: Version, status: i32 },
            #[error("`git push origin -u rc/{tag}` failed. Status code: {status}")]
            PushBranch { tag: Version, status: i32 },
        }
    }

    pub mod github {
        use crate::release::release::git;
        use crate::release::release::Branch;
        use octocrab::models::pulls::PullRequest;
        use octocrab::Error;
        use octocrab::{self, Octocrab};
        use semver::Version;
        use std::{env, process::Command};
        use thiserror::Error;

        pub async fn pr_create(tag: Version) -> Result<PullRequest, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::Octocrab { error }),
            };
            // let octocrab = octocrab::instance();
            let username = get_username();
            let head = format!("rc/{username}_{tag}");
            let base = format!("release/{tag}");
            let title = format!("Release Candidate {tag}");
            let body = title.clone();

            let pr = octocrab
                .pulls("MyceliaNetwork", "mycelia")
                .create(title.clone(), head.clone(), base)
                .body(body)
                .send()
                .await;

            return match pr {
                Ok(pr) => Ok(pr),
                Err(error) => {
                    let _ = git::switch_branch(Branch::Back(&tag));
                    Err(GitHubError::PullRequestCreate { error })
                }
            };

            // return Err(GitHubError::PullRequestCreate {
            //     branch_name: head,
            //     status: -1,
            //     tag,
            // });

            // let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
            // let username = get_username();
            // let branch_name = format!("rc/{username}_{tag}");
            // let github_pr_create_cmd = Command::new(github)
            //     .current_dir(paths::project_root())
            //     .args([
            //         "pr",
            //         "create",
            //         "--assignee",
            //         "@me",
            //         "--fill",
            //         // "--base",
            //         // branch_name.as_str(),
            //         // "--title",
            //         // format!("Release {}", tag.to_string()).as_str(),
            //         // "--body",
            //         // format!("Release {}", tag.to_string()).as_str(),
            //     ])
            //     .status()
            //     .expect("Failed to create GitHub pull request");

            // return match github_pr_create_cmd.code() {
            //     Some(0) => Ok(()),
            //     Some(status) => {
            //         let _ = git::switch_branch(Branch::Back(&tag));
            //         Err(GitHubError::PullRequestCreate {
            //             branch_name,
            //             status,
            //             tag,
            //         })
            //     }
            //     None => Ok(()),
            // };
        }

        pub fn env_token() -> Result<String, GitHubError> {
            // TODO: also test if user has the rights.
            // TODO: Next step would be to hide the `release` feature for users that do not have the rights
            let token = std::env::var("GITHUB_TOKEN");
            return match token {
                Ok(token) => Ok(token),
                _ => Err(GitHubError::EnvToken),
            };
        }

        // pub fn release_create(tag: Version) -> Result<(), GitHubError> {
        //     let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
        //     let username = get_username();
        //     let branch_name = format!("rc/{username}_{tag}");
        //     let github_release_create_cmd = Command::new(github)
        //         .current_dir(paths::project_root())
        //         .args([
        //             "release",
        //             "create",
        //             tag.to_string().as_str(),
        //             "--target",
        //             branch_name.as_str(),
        //             "--prerelease", // TODO: remove this flag when we are ready for a stable release
        //             "--generate-notes",
        //         ])
        //         .status()
        //         .expect("Failed to create GitHub release");

        //     return match github_release_create_cmd.code() {
        //         Some(0) => Ok(()),
        //         Some(status) => {
        //             let _ = git::switch_branch(Branch::Back(&tag));
        //             Err(GitHubError::ReleaseCreate { tag, status })
        //         }
        //         None => Ok(()),
        //     };
        // }

        pub fn get_username() -> String {
            let github = env::var("GH").unwrap_or_else(|_| "gh".to_string());
            let get_username_cmd = Command::new(github)
                .args(["api", "user", "-q", ".login"])
                .output()
                .expect("failed to run `git config user.name`")
                .stdout;

            let user =
                String::from_utf8(get_username_cmd).expect("Failed to convert user name to utf-8");

            user.trim().to_owned()
        }

        #[derive(Debug, Error)]
        pub enum GitHubError {
            #[error("GITHUB_TOKEN environment variable not found. Necessary to create a release")]
            EnvToken,
            #[error("`Octocrab::builder().personal_token(token).build()` failed. Error: {error}")]
            Octocrab { error: Error },
            #[error("octocrab.pulls().create() failed. Error: {error}")]
            PullRequestCreate { error: Error },
            // TODO: update final command
            // #[error(
            //     "`gh release create --prerelease --generate-notes` failed. Status code: {status}"
            // )]
            // ReleaseCreate { tag: Version, status: i32 },
        }
    }
}