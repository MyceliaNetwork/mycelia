use anyhow::Context;

fn main() -> anyhow::Result<()> {
    wit_deps::lock_sync!("../../mycelia_http/wit")
        .context("failed to lock root WIT dependencies")?;
    wit_deps::lock_sync!().context("failed to lock root WIT dependencies")?;

    Ok(())
}
