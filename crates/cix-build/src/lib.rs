//! Workshop engine shared by the Cixfile language surface.
//!
//! This crate owns build-chain execution, lock state, sandboxing, and the Nix
//! expressions that feed those mechanics. `cix-cixfile` owns parsing and the
//! public codegen entry points, and re-exports the language model and lock API.

mod build_chain;
mod codegen;
mod lock;
mod model;
mod seccomp;

pub use build_chain::execute;
pub use codegen::{
    generate_builder_context_nix, generate_builder_offer_nix, generate_fetch_context_nix,
    generate_fetch_offer_nix, generate_nix, generate_nix_with_snapshots, generate_spec_json,
};
pub use lock::{
    ensure_lock, save_lock, ArtifactPin, ConsumedPath, FetchPin, InputLock, LockFile, MemoEntry,
    VolatilePath, DEFAULT_NIXPKGS_URL,
};
pub use model::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, Dirs, Env, Fetch,
    Input, InputKind, Liveness, Port, Probe, Readiness, Service, Template, TemplatePart,
};
