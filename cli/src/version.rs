// FROM: https://github.com/pksunkara/cargo-workspaces/blob/79b446a973292def551b628e807b567653d48242/cargo-workspaces/src/version.rs#L14
use crate::version_opts::Version;
use cargo_metadata::Metadata;
use clap::Parser;

/// Bump version of crates
#[derive(Debug, Parser)]
pub struct Version {
    #[clap(flatten)]
    version: VersionOpt,
}

impl Version {
    pub fn run(self, metadata: Metadata) -> Result {
        self.version.do_versioning(&metadata)?;

        info!("success", "ok");
        Ok(())
    }
}
