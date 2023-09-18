use anyhow::Context;

fn main() -> anyhow::Result<()> {
  wit_deps::lock_sync!().context("failed to lock root WIT dependencies")?;

  Ok(())
}
