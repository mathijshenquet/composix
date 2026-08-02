//! Cixfile parsing, code generation, lock management, and build CLI.

mod build;
pub mod cli;
pub mod fmt;
mod parser;
mod watch;

pub use build::{
    build, build_family, build_family_with_stats, build_with_stats, BuildOptions, BuildStats,
    BuiltItem, StepStat,
};
pub use cix_build::revoke_fetch_consent;
pub use cix_build::{
    ensure_lock, generate_nix, generate_spec_json, ArtifactPin, ConsumedPath, FetchPin, InputLock,
    LockFile, MemoEntry, VolatilePath, DEFAULT_NIXPKGS_URL,
};
pub use cix_build::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, Dirs, Env, Fetch,
    Input, InputKind, Liveness, Port, Probe, Readiness, Secret, Service, Template, TemplatePart,
};
pub use parser::{parse, ParseError};
pub use watch::watch;
