//! Cixfile v1 parsing, Nix code generation, lock management, and build CLI.

mod build;
pub mod cli;
mod codegen;
mod lock;
mod model;
mod parser;

pub use build::{build, BuildOptions};
pub use codegen::{generate_nix, generate_spec_json};
pub use lock::{ensure_lock, LockFile, NixpkgsLock, DEFAULT_NIXPKGS_URL};
pub use model::{Cixfile, Dirs, Env, Item, Port, Service, Template, TemplatePart};
pub use parser::{parse, ParseError};
