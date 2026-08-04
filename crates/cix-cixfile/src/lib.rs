//! Cixfile parsing, code generation, lock management, and build CLI.

mod build;
pub mod cli;
pub mod fmt;
mod parser;
mod watch;

pub use build::{
    build, build_family, build_family_with_stats, build_family_with_stats_file, build_with_stats,
    BuildOptions, BuildStats, BuiltItem, StepStat,
};
pub use cix_build::revoke_fetch_consent;
pub use cix_build::{
    ensure_lock, generate_nix, generate_spec_json, ArtifactPin, ConsumedPath, FetchPin, InputLock,
    LockFile, MemoEntry, VolatilePath, DEFAULT_NIXPKGS_URL,
};
pub use cix_build::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Claim, Copy, CopyMode, Dirs,
    Env, Fetch, Input, InputKind, Liveness, Port, PortSource, Probe, Protocol, Readiness, Secret,
    Service, Template, TemplatePart,
};
pub use parser::{parse, ParseError};
pub use watch::{watch, WatchOptions};

pub fn default_workspace_directory() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".cache"))
        .join("cix/workspaces")
}
