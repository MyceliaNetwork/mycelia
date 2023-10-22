#[allow(clippy::all)]
pub mod release {
    use crate::paths::paths;
    use log::error;

    type DynError = Box<dyn std::error::Error>;

    pub enum Branch<'a> {
        Back,
        Name(&'a String),
    }

    impl<'a> ToString for Branch<'a> {
        fn to_string(&self) -> String {
            match self {
                Branch::Back => "-".to_string(),
                Branch::Name(s) => (*s).clone(),
            }
        }
    }

    pub async fn release() -> Result<(), DynError> {
        if let Err(error) = try_release().await {
            error!("{error:#}");

            std::process::exit(-1);
        }

        Ok(())
    }

    async fn try_release() -> Result<(), DynError> {
        let tag_pre_bump = workspace::parse_cargo_pkg_version();

        github::env_token()?;
        git::status()?;
        workspace::bump()?;

        let tag = workspace::parse_cargo_pkg_version();

        workspace::replace_all_in_file(
            paths::file_rustwrap(),
            "__VERSION__",
            tag.to_string().as_str(),
        );
        let username = github::get_username().await?;
        let branch_name = format!("rc/{username}_{tag}");

        git::create_branch(tag.clone()).await?;
        git::switch_branch(Branch::Name(&branch_name))?;
        git::add_all(tag.clone())?;
        git::commit(tag.clone())?;
        git::push_branch(branch_name).await?;
        github::create_pr(tag_pre_bump, tag.clone()).await?;
        git::switch_branch(Branch::Back)?;

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
        use log::info;
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

        pub fn checkout_branch(branch_name: String, orphan: String) -> Result<(), GitError> {
            let git: String = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            info!("Checking out {branch_name} ");
            let checkout_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["checkout", "-b", branch_name.as_str(), orphan.as_str()])
                .status()
                .expect(format!("`git checkout -b {branch_name} {orphan}` failed").as_str());

            return match checkout_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(GitError::CheckoutBranch {
                    branch_name,
                    orphan,
                    status,
                }),
                None => Ok(()),
            };
        }

        // git checkout -b new_branch_name origin/existing_branch_name_on_git_hub

        pub async fn create_branch(tag: Version) -> Result<(), GitError> {
            let git: String = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let username = github::get_username()
                .await
                .expect("Could not retrieve GitHub username");
            let branch_name = format!("rc/{username}_{tag}");
            info!("Creating branch {branch_name}");
            let create_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["branch", branch_name.as_str()])
                .status()
                .expect(format!("`git branch {branch_name}` failed").as_str());

            return match create_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => Err(GitError::CreateBranch {
                    branch_name,
                    status,
                }),
                None => Ok(()),
            };
        }

        pub fn switch_branch(branch: Branch) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
            let branch_name = branch.to_string();
            let git_switch_branch_cmd = Command::new(git)
                .current_dir(paths::project_root())
                .args(["switch", branch_name.as_str()])
                .status()
                .expect(format!("Failed to run `git switch {branch_name}").as_str());

            return match git_switch_branch_cmd.code() {
                Some(0) => Ok(()),
                Some(status) => {
                    let branch_already_exists: i32 = 128;
                    if status == branch_already_exists {
                        switch_branch(Branch::Back)?;
                    }
                    return Err(GitError::SwitchBranch {
                        branch_name,
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

        pub async fn push_branch(branch_name: String) -> Result<(), GitError> {
            let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());
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
                        switch_branch(Branch::Back)?;
                    }
                    return Err(GitError::PushBranch {
                        branch_name: branch_name.to_string(),
                        status,
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
            #[error("`git branch {branch_name}` failed. Status code: {status}")]
            CreateBranch { branch_name: String, status: i32 },
            #[error("`git switch {branch_name}` failed. Status code: {status}")]
            SwitchBranch { branch_name: String, status: i32 },
            #[error("`git add .` failed for tag {tag}. Status code: {status}")]
            AddAll { tag: Version, status: i32 },
            #[error("`commit -m \"Release {tag:}\"` failed. Status code: {status:}")]
            Commit { tag: Version, status: i32 },
            #[error("`git push origin -u {branch_name}` failed. Status code: {status}")]
            PushBranch { branch_name: String, status: i32 },
            #[error("`git checkout -b {branch_name} {orphan}` failed. Status code: {status}")]
            CheckoutBranch {
                branch_name: String,
                orphan: String,
                status: i32,
            },
        }
    }

    pub mod github {
        use crate::release::release::git;
        use crate::release::release::Branch;
        use octocrab::{
            self,
            models::{pulls::PullRequest, repos::Ref},
            params::repos::Reference,
            Error, Octocrab,
        };
        use semver::Version;
        use thiserror::Error;

        pub async fn create_ref(
            tag: Version,
            target_commit_sha: String,
        ) -> Result<Ref, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            };

            let git_ref = octocrab
                .repos("MyceliaNetwork", "mycelia")
                .create_ref(&Reference::Tag(tag.to_string()), target_commit_sha)
                .await;

            return match git_ref {
                Ok(git_ref) => Ok(git_ref),
                Err(error) => Err(GitHubError::CreateRef { error }),
            };
        }

        pub async fn create_pr(
            tag_pre_bump: Version,
            tag_post_bump: Version,
        ) -> Result<PullRequest, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            };
            let username = get_username()
                .await
                .expect("Could not retrieve GitHub username");
            let base = format!("release/{tag_post_bump}");
            let head = format!("rc/{username}_{tag_post_bump}");
            let title = format!("Release Candidate {tag_post_bump}");
            let body = title.clone();

            let orphan = format!("origin/release/{tag_pre_bump}");
            let create_branch =
                git::checkout_branch(base.clone(), orphan).expect("Checkout branch failed");
            git::push_branch(base.clone())
                .await
                .expect("Pushing release branch as PR head ({head}) failed");
            git::switch_branch(Branch::Back)
                .expect("Switching back branch after Release branch creation failed");

            let pr = octocrab
                .pulls("MyceliaNetwork", "mycelia")
                .create(title.clone(), head.clone(), base.as_str())
                .body(body)
                .send()
                .await;

            return match pr {
                Ok(pr) => Ok(pr),
                Err(error) => {
                    let _ = git::switch_branch(Branch::Back);
                    Err(GitHubError::CreatePullRequest { head, base, error })
                }
            };
        }

        pub fn env_token() -> Result<String, GitHubError> {
            // TODO: Next step would be to hide the `release` feature for users that do not have the rights
            let token = std::env::var("GITHUB_TOKEN");
            return match token {
                Ok(token) => Ok(token),
                _ => Err(GitHubError::EnvTokenNotFound),
            };
        }

        pub async fn get_username() -> Result<String, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            };

            let author = octocrab.current().user().await;

            return match author {
                Ok(author) => Ok(author.login),
                Err(_) => Err(GitHubError::EnvTokenInvalid),
            };
        }

        #[derive(Debug, Error)]
        pub enum GitHubError {
            #[error("GITHUB_TOKEN environment variable not found. IMPORTANT: add it to the .gitignored /.env file in the project root to make sure your secrets do not leak.")]
            EnvTokenNotFound,
            #[error("Bad GitHub credentials. Please check if the GITHUB_TOKEN in your /.env file is correctly configured and has the required permissions.")]
            EnvTokenInvalid,
            #[error("`octocrab.repos().create_ref()` failed. Error: {error}")]
            CreateRef { error: Error },
            #[error("`Octocrab::builder().personal_token(token).build()` failed. Error: {error}")]
            OctocrabTokenBuild { error: Error },
            #[error("`octocrab.pulls().create()` failed. {head} -> {base} Error: {error}")]
            CreatePullRequest {
                head: String,
                base: String,
                error: Error,
            },
        }
    }
}
