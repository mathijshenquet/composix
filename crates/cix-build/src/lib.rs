//! Workshop engine shared by the Cixfile language surface.
//!
//! This crate owns build-chain execution, lock state, sandboxing, and the Nix
//! expressions that feed those mechanics. `cix-cixfile` owns parsing and the
//! public codegen entry points, and re-exports the language model and lock API.
//!
//! ## Module map
//!
//! - `build_chain`: conducts ordered FETCH/BUILDER dispatch and assembles receipts.
//! - `evaluation`: owns typed Nix-evaluation requests and results.
//! - `eval_plan`: records pure Cixfile evaluation.
//! - `fetch`: owns credential consent and fetch inputs.
//! - `fetch_state`: owns FETCH snapshots, volatility, and pin refresh.
//! - `fhs`: diagnoses FHS-loader compatibility.
//! - `lock`: owns persisted pins and memo records.
//! - `memo`: owns build-step keys, validation, reduction, and constructive replay.
//! - `model`: defines the shared Cixfile language model.
//! - `scratch`: owns temporary build-state lifecycle.
//! - `seccomp`: owns build network policy.
//! - `sandbox`: owns typed traced-sandbox requests and results.
//! - `trace`: captures build read sets.
//! - `workspace`: owns persisted and disposable builder filesystem state.
//!
//! New build feature strata belong in their own module.

/// Whether `CIX timing …` instrumentation lines are emitted on stderr.
/// Opt-in via the CIX_TIMING env var so the measurement harness gets its
/// receipts while ordinary build output (and the generated tour) stays clean.
// Process-wide instrumentation flag: a OnceLock static is the canonical
// acceptable use per AGENTS.md (read-once env, no mutation after init).
pub(crate) fn timing_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("CIX_TIMING").is_some())
}

macro_rules! cix_timing {
    ($($arg:tt)*) => {
        if $crate::timing_enabled() {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use cix_timing;

mod build_chain;
mod eval_plan;
mod evaluation;
mod fetch;
mod fetch_state;
mod fhs;
mod lock;
mod memo;
mod model;
mod sandbox;
mod scratch;
mod seccomp;
mod trace;
mod workspace;

pub use build_chain::execute;
pub use eval_plan::{EvalPlan, EVAL_PLAN_VERSION};
pub use evaluation::EvaluationCodegen;
pub use fetch::revoke_fetch_consent;
pub use lock::{
    ensure_lock, resolve_input_metadata, save_lock, validate_declared_expectations, ArtifactPin,
    ArtifactResolver, ConsumedPath, DevEnvironment, FetchPin, InputLock, LockFile, MemoEntry,
    OutputHash, OutputReceipt, ReadDependency, StepChange, StepMemo, VolatilePath,
    DEFAULT_NIXPKGS_URL,
};
pub use memo::ExecutedStep;
pub use scratch::{
    configure as configure_scratch, install_signal_cleanup, sweep_stale as sweep_stale_scratch,
    ScratchDir,
};

pub const BUILDER_FINGERPRINT: &str = concat!(env!("CARGO_PKG_VERSION"), ":d87-v2");
pub use model::{
    Arg, Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, CopyMode,
    Dirs, Env, Fetch, Input, InputKind, Liveness, NodeCommand, Port, PortSource, Probe, Protocol,
    Readiness, Secret, Service, Template, TemplatePart,
};
