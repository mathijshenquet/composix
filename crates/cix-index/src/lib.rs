//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See docs/design.md "Part 1 — index".
//!
//! ## Module map
//!
//! `refs` owns the current table/pointer state, while `tags`, `roots`, `pull`,
//! and `serve` own their respective index operations. New index feature strata
//! belong in their own module.

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
