//! Shared types and nix interop for composix.
//!
//! Ownership: the index track owns ref parsing and the local tag store; the
//! run track owns spec types. Genuinely shared pieces (store path handling,
//! `nix` subprocess helpers) live here.
