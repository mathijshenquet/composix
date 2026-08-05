//! Part 2: cix-manifest.json parsing/validation, systemd unit generation, cix run.
//! See docs/design.md "Part 2 — spec + run".
//!
//! ## Module map
//!
//! `runtime` validates run options and conducts service selection. `target`
//! resolves paths, refs, and Nix installables; `app` runs finite or scheduled
//! apps; `manager` owns persistent units, listeners, GC roots, and journal
//! control. `unit` conducts ordered systemd property assembly; `devices`,
//! `directories`, `health`, and `closed_root` each own one feature projection.

mod app;
pub mod capabilities;
pub mod cli;
pub mod closed_root;
pub mod config;
pub mod debug;
mod devices;
mod directories;
pub mod exec;
mod health;
mod manager;
pub mod probe;
pub mod runtime;
pub mod shell;
pub mod spec;
mod target;
pub mod unit;
