use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // This isn't actually being used. But, serves as a future example
    // of how to include local wits
    wit_deps::lock_sync!("../../mycelia_http/wit")
        .context("failed to lock root WIT dependencies")?;
    wit_deps::lock_sync!().context("failed to lock root WIT dependencies")?;

    Ok(())
}
