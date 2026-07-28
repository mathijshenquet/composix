//! Cixfile v1 parsing, Nix code generation, lock management, and build CLI.

mod codegen;
mod lock;
mod model;
mod parser;

pub use codegen::{generate_nix, generate_spec_json};
pub use lock::{LockFile, NixpkgsLock};
pub use model::{Cixfile, Dirs, Env, Item, Port, Service, Template, TemplatePart};
pub use parser::{parse, ParseError};
