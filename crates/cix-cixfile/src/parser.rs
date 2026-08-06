//! Thin Cixfile parser facade; implementation is split by parsing concern.

mod diagnostics;
mod directives;
mod machine;
mod migrations;
mod validate;

pub use machine::{parse, parse_with_args, ParseError};
