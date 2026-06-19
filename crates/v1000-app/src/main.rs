//! V1000 editor entry point.

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    v1000_gui::run()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to run the V1000 editor")
}
