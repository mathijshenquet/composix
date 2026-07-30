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
    Assembly, BuildStep, Cixfile, Dirs, Env, Input, Item, Port, Service, Take, Template,
    TemplatePart,
};
pub use parser::{parse, ParseError};
