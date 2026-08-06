//! Declared build-evidence exclusions and conservative authoring hints.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};

pub(crate) fn normalize_paths(paths: &BTreeSet<String>) -> Result<BTreeSet<String>> {
    paths.iter().map(|path| normalize_path(path)).collect()
}

fn normalize_path(path: &str) -> Result<String> {
    let input = Path::new(path);
    let relative = if input.is_absolute() {
        input.strip_prefix("/work").map_err(|_| {
            anyhow::anyhow!(
                "WITH UNSAFE IGNORE path {path:?} is outside /work; name a workspace-relative path or /work/<path>; see docs/cixfile.md#unsafe-ignore"
            )
        })?
    } else {
        input
    };
    let mut normalized = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => bail!(
                "WITH UNSAFE IGNORE path {path:?} must stay within /work and cannot contain ..; see docs/cixfile.md#unsafe-ignore"
            ),
        }
    }
    Ok(if normalized.as_os_str().is_empty() {
        ".".into()
    } else {
        normalized.to_string_lossy().into_owned()
    })
}

pub(crate) fn report_waivers(kind: &str, line: usize, source: &str, paths: &BTreeSet<String>) {
    for path in paths {
        eprintln!("{}", waiver_message(kind, line, source, path));
    }
}

fn waiver_message(kind: &str, line: usize, source: &str, path: &str) -> String {
    format!(
        "warning: line {line}: {kind} waives build evidence under {path:?}; this subtree is excluded from reads, seals, pins, and keys\n  | {source:?}\n  see docs/cixfile.md#unsafe-ignore"
    )
}

pub(crate) fn retain_included<T>(values: &mut BTreeMap<String, T>, excluded: &BTreeSet<String>) {
    values.retain(|path, _| !overlaps_any(path, excluded));
}

pub(crate) fn retain_included_set(values: &mut BTreeSet<String>, excluded: &BTreeSet<String>) {
    values.retain(|path| !overlaps_any(path, excluded));
}

pub(crate) fn report_candidates(kind: &str, line: usize, candidates: &BTreeSet<String>) {
    for path in candidates {
        eprintln!("{}", candidate_message(kind, line, path));
    }
}

fn candidate_message(kind: &str, line: usize, path: &str) -> String {
    format!(
        "hint: line {line}: {kind} wrote and read cache-shaped subtree {path:?}; if it is load-bearing volatile state with an author-maintained invariant, consider `WITH UNSAFE IGNORE {path}`; cix will never add this clause automatically; see docs/cixfile.md#unsafe-ignore"
    )
}

pub(crate) fn overlaps_any(path: &str, roots: &BTreeSet<String>) -> bool {
    roots
        .iter()
        .any(|root| same_or_descendant(path, root) || same_or_descendant(root, path))
}

pub(crate) fn same_or_descendant(candidate: &str, root: &str) -> bool {
    root == "."
        || candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_paths_are_workspace_relative_and_cannot_escape() {
        assert_eq!(normalize_path("cache/./store").unwrap(), "cache/store");
        assert_eq!(normalize_path("/work/cache").unwrap(), "cache");
        assert_eq!(
            normalize_path("../cache").unwrap_err().to_string(),
            "WITH UNSAFE IGNORE path \"../cache\" must stay within /work and cannot contain ..; see docs/cixfile.md#unsafe-ignore"
        );
        assert_eq!(
            normalize_path("/tmp/cache").unwrap_err().to_string(),
            "WITH UNSAFE IGNORE path \"/tmp/cache\" is outside /work; name a workspace-relative path or /work/<path>; see docs/cixfile.md#unsafe-ignore"
        );
        assert_eq!(
            waiver_message("RUN", 4, "RUN tool", "cache"),
            "warning: line 4: RUN waives build evidence under \"cache\"; this subtree is excluded from reads, seals, pins, and keys\n  | \"RUN tool\"\n  see docs/cixfile.md#unsafe-ignore"
        );
        assert_eq!(
            candidate_message("FETCH", 7, "go/pkg/mod"),
            "hint: line 7: FETCH wrote and read cache-shaped subtree \"go/pkg/mod\"; if it is load-bearing volatile state with an author-maintained invariant, consider `WITH UNSAFE IGNORE go/pkg/mod`; cix will never add this clause automatically; see docs/cixfile.md#unsafe-ignore"
        );
    }
}
