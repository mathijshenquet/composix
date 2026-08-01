//! Cixfile parsing, code generation, lock management, and build CLI.

mod build;
pub mod cli;
pub mod fmt;
mod parser;
mod watch;

pub use build::{build, build_family, BuildOptions, BuiltItem};
pub use cix_build::{
    ensure_lock, generate_nix, generate_spec_json, ArtifactPin, ConsumedPath, FetchPin, InputLock,
    LockFile, MemoEntry, VolatilePath, DEFAULT_NIXPKGS_URL,
};
pub use cix_build::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Copy, Dirs, Env, Fetch, Input,
    InputKind, Port, Service, Template, TemplatePart,
};
pub use parser::{parse, ParseError};
pub use watch::watch;
