#[allow(clippy::all)]
pub mod rc {
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

    pub async fn rc() -> Result<(), DynError> {
        if let Err(error) = try_rc().await {
            error!("{error:#}");

            std::process::exit(-1);
        }

        Ok(())
    }

    async fn try_rc() -> Result<(), DynError> {
        let tag_pre_bump = workspace::parse_cargo_pkg_version();

        github::env_token()?;
        git::status()?;
        workspace::bump()?;

        let tag_post_bump = workspace::parse_cargo_pkg_version();

        workspace::replace_all_in_file(
            paths::file_rustwrap(),
            "__VERSION__",
            tag_post_bump.to_string().as_str(),
        );
        let username = github::get_username().await?;
        let branch_name = format!("rc/{username}_{tag_post_bump}");

        github::create_tag(tag_pre_bump.clone(), tag_post_bump.clone()).await?;
        git::create_branch(tag_post_bump.clone()).await?;
        git::switch_branch(Branch::Name(&branch_name))?;
        git::add_all(tag_post_bump.clone())?;
        git::commit(tag_post_bump.clone())?;
        git::push_branch(branch_name).await?;
        git::switch_branch(Branch::Back);
        github::create_pr(tag_pre_bump.clone(), tag_post_bump.clone()).await?;

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
        use crate::rc::rc::github;
        use crate::rc::rc::Branch;
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
            let commit_msg = format!("Release Candidate {tag}");
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
            #[error("`commit -m \"Release Candidate {tag:}\"` failed. Status code: {status:}")]
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
        use crate::rc::rc::git;
        use crate::rc::rc::Branch;
        use log::info;
        use octocrab::{
            self,
            models::{
                pulls::PullRequest,
                repos::{Object, Object::Commit, Ref},
            },
            params::repos::Reference,
            Error, Octocrab,
        };
        use semver::Version;
        use thiserror::Error;

        async fn get_ref<'a>(branch: Branch<'a>) -> Result<Ref, GitHubError> {
            let branch = &Reference::Branch(branch.to_string());
            let git_ref = octocrab::instance()
                .repos("MyceliaNetwork", "mycelia")
                .get_ref(branch)
                .await;

            return match git_ref {
                Ok(git_ref) => Ok(git_ref),
                Err(error) => return Err(GitHubError::CouldNotGetRef { error }),
            };
        }

        async fn create_ref<'a>(
            branch: Branch<'a>,
            target_commit_sha: String,
        ) -> Result<Ref, GitHubError> {
            let token = env_token()?;
            let octocrab = Octocrab::builder().personal_token(token).build();
            let octocrab = match octocrab {
                Ok(octocrab) => octocrab,
                Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
            };

            let tag = Reference::Tag(branch.to_string());
            let git_ref = octocrab
                .repos("MyceliaNetwork", "mycelia")
                .create_ref(&tag, target_commit_sha)
                .await;

            return match git_ref {
                Ok(git_ref) => Ok(git_ref),
                Err(error) => Err(GitHubError::CreateRef { error }),
            };
        }

        pub fn sha_from_ref(git_ref: Ref) -> Result<String, GitHubError> {
            let object: Object = git_ref.object;

            return match object {
                Commit { sha, .. } => Ok(sha),
                _ => Err(GitHubError::ShaNotFound),
            };
        }

        // pub async fn merge_branch<'a>(
        //     head: Branch<'a>,
        //     base: Branch<'a>,
        // ) -> Result<MergeCommit, GitHubError> {
        //     let head = head.to_string();
        //     let base = base.to_string();
        //     let token = env_token()?;
        //     let octocrab = Octocrab::builder().personal_token(token).build();
        //     let octocrab = match octocrab {
        //         Ok(octocrab) => octocrab,
        //         Err(error) => return Err(GitHubError::OctocrabTokenBuild { error }),
        //     };
        //     let username = get_username()
        //         .await
        //         .expect("Could not retrieve GitHub username");

        //     let commit_msg = format!("Merge {base} into {head} to be merged into for release");
        //     info!("{commit_msg}");

        //     let merge_commit = octocrab
        //         .repos("MyceliaNetwork", "mycelia")
        //         .merge(head.clone(), base.clone())
        //         .commit_message(commit_msg)
        //         .send()
        //         .await;

        //     return match merge_commit {
        //         Ok(commit) => Ok(commit),
        //         Err(error) => {
        //             let _ = git::switch_branch(Branch::Back);
        //             Err(GitHubError::MergeCommit { base, head, error })
        //         }
        //     };
        // }

        pub async fn create_pr(
            tag_pre_bump: Version,
            tag_post_bump: Version,
        ) -> Result<PullRequest, GitHubError> {
            info!("Creating RC PR to update {tag_pre_bump} to {tag_post_bump}");
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
            git::checkout_branch(base.clone(), orphan).expect("Checkout branch failed");
            git::push_branch(base.clone())
                .await
                .expect("Pushing RC branch as PR head ({head}) failed");
            git::switch_branch(Branch::Back)
                .expect("Switching back branch after RC branch creation failed");

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

        pub async fn create_tag(
            tag_pre_bump: Version,
            tag_post_bump: Version,
        ) -> Result<(), GitHubError> {
            let prev_release_branch_name = format!("release/{tag_pre_bump}");
            let prev_release_branch = Branch::Name(&prev_release_branch_name);
            let prev_release_git_ref = get_ref(prev_release_branch).await?;
            let prev_release_sha = sha_from_ref(prev_release_git_ref).expect("SHA conversion");

            let release_branch_name = format!("release/{tag_post_bump}");
            let release_branch = Branch::Name(&release_branch_name);
            create_ref(release_branch, prev_release_sha)
                .await
                .expect("Ref Creation");
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
            #[error("Ref not found. Error {error}")]
            CouldNotGetRef { error: Error },
            #[error("Ref SHA not found")]
            ShaNotFound,
            #[error("`octocrab.repos().create_ref()` failed. Error: {error}")]
            CreateRef { error: Error },
            #[error("`Octocrab::builder().personal_token(token).build()` failed. Error: {error}")]
            OctocrabTokenBuild { error: Error },
            // #[error("`octocrab.repos().merge()` failed: {base} -> {head}  Error: {error}")]
            // MergeCommit {
            //     base: String,
            //     head: String,
            //     error: Error,
            // },
            #[error("`octocrab.pulls().create()` failed. {head} -> {base} Error: {error}")]
            CreatePullRequest {
                head: String,
                base: String,
                error: Error,
            },
        }
    }
}
