//! Part 2: cix-manifest.json parsing/validation, systemd unit generation, cix run.
//! See docs/design.md "Part 2 — spec + run".

pub mod capabilities;
pub mod cli;
pub mod closed_root;
pub mod config;
pub mod debug;
mod devices;
mod directories;
pub mod exec;
mod health;
pub mod probe;
pub mod runtime;
pub mod shell;
pub mod spec;
pub mod unit;
