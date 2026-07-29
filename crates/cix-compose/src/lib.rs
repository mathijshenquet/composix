//! Compose v0: strict manifests, locked resolution, deterministic generations, and activation.

pub mod cli;
mod model;
mod resolve;

pub use model::{Compose, Lock};
pub use resolve::{load_and_check, CheckResult, UpdateRequest};

pub fn ps() -> anyhow::Result<()> {
    anyhow::bail!("compose ps is not implemented yet")
}
