//! Compose v0: strict manifests, locked resolution, deterministic generations, and activation.

pub mod cli;
mod generation;
mod model;
mod ps;
mod resolve;
mod runtime;

pub use generation::{build_generation, render_generation, BuiltGeneration, Manifest};
pub use model::{Compose, Lock};
pub use resolve::{load_and_check, CheckResult, UpdateRequest};
pub use runtime::{check, diff, down, rollback, up};

pub use ps::ps;
