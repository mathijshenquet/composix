//! Compose trees: strict manifests, locked resolution, deterministic generations, and activation.

pub mod cli;
mod generation;
mod model;
mod observability;
mod ps;
mod resolve;
mod runtime;

pub use generation::{
    build_generation, build_generation_with_closed_root, render_generation,
    render_generation_with_closed_root, BuiltGeneration, Manifest, ManifestDegradation,
};
pub use model::{Child, Compose, ComposeService, Group, Lock};
pub use resolve::{load_and_check, CheckResult, UpdateRequest};
pub use runtime::{check, clean, diff, down, rollback, up};

pub use observability::{logs, result_label, stats, LogsOptions};
pub use ps::{ps, render_ps_table, PsRow};

pub(crate) fn unit_path(path: &str) -> String {
    path.bytes().fold(String::new(), |mut escaped, byte| {
        match byte {
            b'/' => escaped.push('-'),
            b'-' => escaped.push_str("\\x2d"),
            _ => escaped.push(char::from(byte)),
        }
        escaped
    })
}

#[cfg(test)]
mod tests {
    use super::unit_path;

    #[test]
    fn unit_paths_preserve_component_boundaries() {
        assert_eq!(unit_path("tier/api"), "tier-api");
        assert_eq!(unit_path("tier-api"), r"tier\x2dapi");
        assert_ne!(unit_path("a-b/c"), unit_path("a/b-c"));
    }
}
