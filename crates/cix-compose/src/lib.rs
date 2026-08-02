//! Compose v0: strict manifests, locked resolution, deterministic generations, and activation.

pub mod cli;
mod generation;
mod model;
mod observability;
mod ps;
mod resolve;
mod runtime;

pub use generation::{
    build_generation, build_generation_with_closed_root, render_generation,
    render_generation_with_closed_root, BuiltGeneration, Manifest, ManifestDegradation,
};
pub use model::{Compose, Lock};
pub use resolve::{load_and_check, CheckResult, UpdateRequest};
pub use runtime::{check, clean, diff, down, rollback, up};

pub use observability::{logs, result_label, stats, LogsOptions};
pub use ps::ps;
