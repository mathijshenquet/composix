//! Workshop engine shared by the Cixfile language surface.
//!
//! This crate owns build-chain execution, lock state, sandboxing, and the Nix
//! expressions that feed those mechanics. `cix-cixfile` owns parsing and the
//! public codegen entry points, and re-exports the language model and lock API.
//!
//! ## Module map
//!
//! `build_chain` conducts step ordering and sandbox execution; `fetch` owns
//! credential consent; `trace` owns read-set capture; `lock` owns persisted
//! pins and memo records. New build feature strata belong in their own module.

mod build_chain;
mod codegen;
mod fetch;
mod lock;
mod model;
mod seccomp;
mod trace;

pub use build_chain::{execute, ExecutedStep};
pub use codegen::{
    generate_builder_context_nix, generate_builder_dev_env_nix, generate_builder_offer_nix,
    generate_fetch_context_nix, generate_fetch_offer_nix, generate_nix,
    generate_nix_with_snapshots, generate_spec_json,
};
pub use fetch::revoke_fetch_consent;
pub use lock::{
    ensure_lock, resolve_input_metadata, save_lock, ArtifactPin, ConsumedPath, DevEnvironment,
    FetchPin, InputLock, LockFile, MemoEntry, OutputReceipt, ReadDependency, StepChange, StepMemo,
    VolatilePath, DEFAULT_NIXPKGS_URL,
};

pub const BUILDER_FINGERPRINT: &str = concat!(env!("CARGO_PKG_VERSION"), ":d87-v2");
pub use model::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, Dirs, Env, Fetch,
    Input, InputKind, Liveness, Port, Probe, Readiness, Secret, Service, Template, TemplatePart,
};
