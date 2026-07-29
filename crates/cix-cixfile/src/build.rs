use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{ensure_lock, generate_nix, parse};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub directory: PathBuf,
    pub update_lock: Option<String>,
    pub tag: Option<String>,
}

pub fn build(options: &BuildOptions) -> Result<String> {
    let directory = options
        .directory
        .canonicalize()
        .with_context(|| format!("resolving build directory {}", options.directory.display()))?;
    let cixfile_path = directory.join("Cixfile");
    let source = fs::read_to_string(&cixfile_path)
        .with_context(|| format!("reading {}", cixfile_path.display()))?;
    let cixfile = parse(&source).with_context(|| format!("parsing {}", cixfile_path.display()))?;
    let lock = ensure_lock(
        &directory.join("Cixfile.lock"),
        &cixfile.inputs,
        options.update_lock.as_deref(),
    )?;
    let system = cix_common::current_system()?;
    let expression = generate_nix(&cixfile, &directory, &lock, &system)?;
    let store_path = build_expression(&expression)?;
    if let Some(tag) = &options.tag {
        cix_index::tag(&store_path, tag, None)
            .with_context(|| format!("tagging built item as {tag:?}"))?;
    }
    Ok(store_path)
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
