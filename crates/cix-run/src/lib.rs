//! Part 2: cix-manifest.json parsing/validation, systemd unit generation, cix run.
//! See docs/design.md "Part 2 — spec + run".

pub mod capabilities;
pub mod cli;
pub mod config;
pub mod debug;
pub mod exec;
pub mod probe;
pub mod runtime;
pub mod shell;
pub mod spec;
pub mod unit;
