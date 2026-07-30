use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::build_chain;
use crate::codegen::generate_nix_with_snapshot;
use crate::lock::save_lock;
use crate::{ensure_lock, parse};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub directory: PathBuf,
    pub update_lock: Option<String>,
    pub tag: Option<String>,
    pub no_cache: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltItem {
    pub name: String,
    pub store_path: String,
}

pub fn build(options: &BuildOptions) -> Result<Vec<BuiltItem>> {
    let directory = options
        .directory
        .canonicalize()
        .with_context(|| format!("resolving build directory {}", options.directory.display()))?;
    let cixfile_path = directory.join("Cixfile");
    let source = fs::read_to_string(&cixfile_path)
        .with_context(|| format!("reading {}", cixfile_path.display()))?;
    let cixfile = parse(&source).with_context(|| format!("parsing {}", cixfile_path.display()))?;
    if let Some(tag) = &options.tag {
        for name in cixfile.items.keys() {
            tag_reference(cixfile.items.len(), name, tag)?;
        }
    }
    let lock_path = directory.join("Cixfile.lock");
    let mut lock = ensure_lock(&lock_path, &cixfile.inputs, options.update_lock.as_deref())?;
    let system = cix_common::current_system()?;
    let build_snapshot = build_chain::execute(
        &cixfile,
        &directory,
        &mut lock,
        &system,
        options.update_lock.as_deref() == Some(""),
        options.no_cache,
    );
    save_lock(&lock_path, &lock)?;
    let build_snapshot = build_snapshot?;
    let mut outputs = Vec::new();
    for name in cixfile.items.keys() {
        let expression = generate_nix_with_snapshot(
            &cixfile,
            name,
            &directory,
            &lock,
            &system,
            build_snapshot.as_deref(),
        )?;
        let realized = build_expression(&expression)?;
        let store_path = add_item_to_store(&realized, name)?;
        if let Some(tag) = &options.tag {
            let reference = tag_reference(cixfile.items.len(), name, tag)?;
            cix_index::tag(&store_path, &reference, None)
                .with_context(|| format!("tagging built ITEM {name:?} as {reference:?}"))?;
        }
        outputs.push(BuiltItem {
            name: name.clone(),
            store_path,
        });
    }
    Ok(outputs)
}

fn tag_reference(item_count: usize, item_name: &str, tag: &str) -> Result<String> {
    if item_count > 1 && tag.contains(':') {
        anyhow::bail!(
            "-t name:tag is ambiguous for a multi-ITEM Cixfile; pass only the tag so each ITEM is tagged as <item-name>:<tag>"
        );
    }
    Ok(if item_count == 1 && tag.contains(':') {
        tag.to_owned()
    } else {
        format!("{item_name}:{tag}")
    })
}

fn build_expression(expression: &str) -> Result<String> {
    let output = cix_common::nix(&[
        "build",
        "--impure",
        "--no-link",
        "--print-out-paths",
        "--expr",
        expression,
    ])?;
    output
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .context("nix build did not print an output store path")
}

fn add_item_to_store(path: &str, name: &str) -> Result<String> {
    let output = cix_common::nix(&[
        "store",
        "add",
        "--mode",
        "nar",
        "--name",
        &format!("cix-item-{name}"),
        path,
    ])?;
    output
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .context("nix store add did not print an item store path")
}

#[cfg(test)]
mod tests {
    use super::tag_reference;

    #[test]
    fn multi_item_tags_are_item_names_and_reject_full_refs() {
        assert_eq!(tag_reference(2, "api", "v7").unwrap(), "api:v7".to_owned());
        let error = tag_reference(2, "api", "other:v7").unwrap_err().to_string();
        assert!(error.contains("multi-ITEM"), "{error}");
        assert_eq!(
            tag_reference(1, "api", "other:v7").unwrap(),
            "other:v7".to_owned()
        );
    }
}
