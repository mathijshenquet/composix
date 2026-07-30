//! Cixfile v1 parsing, Nix code generation, lock management, and build CLI.

mod build;
mod build_chain;
pub mod cli;
mod codegen;
mod lock;
mod model;
mod parser;
mod seccomp;

pub use build::{build, BuildOptions, BuiltItem};
pub use codegen::{generate_nix, generate_spec_json};
pub use lock::{ensure_lock, FetchPin, InputLock, LockFile, MemoEntry, DEFAULT_NIXPKGS_URL};
pub use model::{
    Artifact, ArtifactKind, Assembly, BuildStep, Builder, Cixfile, Copy, Dirs, Env, Fetch, Input,
    InputKind, Port, Service, Template, TemplatePart,
};
pub use parser::{parse, ParseError};
