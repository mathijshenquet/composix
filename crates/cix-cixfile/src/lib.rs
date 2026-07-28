//! Cixfile v1 parsing, Nix code generation, lock management, and build CLI.

mod model;
mod parser;

pub use model::{Cixfile, Dirs, Env, Item, Port, Service, Template, TemplatePart};
pub use parser::{parse, ParseError};
