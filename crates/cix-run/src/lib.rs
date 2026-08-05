//! Part 2: cix-manifest.json parsing/validation, systemd unit generation, cix run.
//! See docs/design.md "Part 2 — spec + run".
//!
//! ## Module map
//!
//! - `app`: runs finite and scheduled apps.
//! - `capabilities`: owns capability policy and host verification.
//! - `cli`: owns run command-line parsing and dispatch.
//! - `closed_root`: owns sealed-root unit projection.
//! - `config`: owns run configuration shapes and validation.
//! - `debug`: owns debug command projection.
//! - `degradation`: owns explicit user-manager degradation reporting.
//! - `devices`: owns device access projection.
//! - `directories`: owns declared-directory projection.
//! - `exec`: owns commands executed in a running service.
//! - `health`: owns health-check projection.
//! - `manager`: owns persistent units, listeners, GC roots, and journal control.
//! - `probe`: owns readiness and liveness probe execution.
//! - `runtime`: validates run options and conducts service selection.
//! - `shell`: owns interactive shell command projection.
//! - `spec`: owns manifest parsing and validation.
//! - `target`: resolves paths, refs, and Nix installables.
//! - `unit`: conducts ordered systemd property assembly.

mod app;
pub mod capabilities;
pub mod cli;
pub mod closed_root;
pub mod config;
pub mod debug;
mod degradation;
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
