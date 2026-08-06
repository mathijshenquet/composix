//! Cixfile parsing, code generation, lock management, and build CLI.
//!
//! ## Module map
//!
//! - `build`: owns build orchestration and its typed options.
//! - `codegen`: projects language plans into Nix and neutral manifests.
//! - `fmt`: owns Cixfile formatting.
//! - `parser`: owns the grammar facade and its parser-internal strata.
//!
//! Parser submodules are intentionally mapped by `parser`, their direct owner;
//! this crate-root map covers every direct module.

mod build;
mod codegen;
pub mod fmt;
mod parser;

pub use build::{
    build, build_family, build_family_with_registry, build_family_with_stats,
    build_family_with_stats_and_registry, build_family_with_stats_file,
    build_family_with_stats_file_and_registry, build_with_registry, build_with_stats,
    build_with_stats_and_registry, ArtifactRegistry, BuildOptions, BuildStats, BuiltItem, StepStat,
};
pub use cix_build::revoke_fetch_consent;
pub use cix_build::{
    ensure_lock, ArtifactPin, ConsumedPath, FetchPin, InputLock, LockFile, MemoEntry, VolatilePath,
    DEFAULT_NIXPKGS_URL,
};
pub use cix_build::{
    Arg, Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, CopyMode,
    Dirs, Env, Fetch, Input, InputKind, Liveness, NodeCommand, Port, PortSource, Probe, Protocol,
    Readiness, Secret, Service, Template, TemplatePart,
};
pub use codegen::{generate_nix, generate_nix_with_snapshots, generate_spec_json};
pub use parser::{parse, ParseError};

pub fn default_workspace_directory() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
        .join("cix/workspaces")
}
