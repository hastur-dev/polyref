use anyhow::{Result, Context, anyhow, bail};
use anyhow::NonExistent;  // ERROR: not a real export

fn main() -> Result<()> {
    let err = anyhow!("error");
    bail!("fail");

    // This should be caught: wrong method name
    err.nonexistent_method();

    // This is valid
    err.context("adding context");

    Ok(())
}
