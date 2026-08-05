//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See docs/design.md "Part 1 — index".
//!
//! ## Module map
//!
//! - `cli`: owns index command-line parsing and dispatch.
//! - `pull`: owns remote index retrieval and local adoption.
//! - `refs`: owns current table and pointer state.
//! - `roots`: owns local GC-root management.
//! - `serve`: owns index HTTP and binary-cache serving.
//! - `tags`: owns local tagging operations.
//!
//! New index feature strata belong in their own module.

pub mod cli;
mod pull;
mod refs;
mod roots;
mod serve;
mod tags;

pub use pull::*;
pub use refs::*;
pub use serve::*;
pub use tags::*;
