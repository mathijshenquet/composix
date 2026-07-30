use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::build_chain;
use crate::codegen::generate_nix_with_snapshots;
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
    if let Some(requested) = options.update_lock.as_deref() {
        reject_expected_fetch_update(&cixfile, requested)?;
    }
    if let Some(tag) = &options.tag {
        for name in &cixfile.artifact_order {
            tag_reference(cixfile.artifacts.len(), name, tag)?;
        }
    }
    let requested_update = options.update_lock.as_deref();
    let input_update = match requested_update {
        Some("") | None => requested_update,
        Some(name)
            if cixfile
                .inputs
                .get(name)
                .is_some_and(|input| !input.is_local()) =>
        {
            Some(name)
        }
        Some(name) if cixfile.fetches.contains_key(name) || cixfile.builders.contains_key(name) => {
            None
        }
        Some(name)
            if cixfile
                .inputs
                .get(name)
                .is_some_and(|input| input.is_local()) =>
        {
            anyhow::bail!("FROM . AS {name} is the local build context and is not lock-pinned")
        }
        Some(name) => anyhow::bail!(
            "--update-lock names no lock-bearing FROM, FETCH, or BUILDER binder {name:?}"
        ),
    };
    let lock_path = directory.join("Cixfile.lock");
    let mut lock = ensure_lock(&lock_path, &cixfile.inputs, input_update)?;
    let system = cix_common::current_system()?;
    let snapshots = build_chain::execute(
        &cixfile,
        &directory,
        &mut lock,
        &system,
        requested_update,
        options.no_cache,
    );
    save_lock(&lock_path, &lock)?;
    let snapshots = snapshots?;
    let mut outputs = Vec::new();
    for name in &cixfile.artifact_order {
        let expression =
            generate_nix_with_snapshots(&cixfile, name, &directory, &lock, &system, &snapshots)?;
        let realized = build_expression(&expression)?;
        let store_path = add_item_to_store(&realized, name)?;
        if let Some(tag) = &options.tag {
            let reference = tag_reference(cixfile.artifacts.len(), name, tag)?;
            cix_index::tag(&store_path, &reference, None)
                .with_context(|| format!("tagging built artifact {name:?} as {reference:?}"))?;
        }
        outputs.push(BuiltItem {
            name: name.clone(),
            store_path,
        });
    }
    Ok(outputs)
}

fn reject_expected_fetch_update(cixfile: &crate::Cixfile, requested: &str) -> Result<()> {
    let top_level = cixfile.fetch_order.iter().find(|name| {
        (requested.is_empty() || requested == name.as_str())
            && cixfile.fetches[*name].expected.is_some()
    });
    if let Some(name) = top_level {
        anyhow::bail!(
            "--update-lock is meaningless for FETCH {name:?} with EXPECT; change the EXPECT value instead"
        );
    }
    let builder = cixfile.builder_order.iter().find(|name| {
        (requested.is_empty() || requested == name.as_str())
            && cixfile.builders[*name].steps.iter().any(|step| {
                matches!(
                    step,
                    crate::BuildStep::Fetch {
                        expected: Some(_),
                        ..
                    }
                )
            })
    });
    if let Some(name) = builder {
        anyhow::bail!(
            "--update-lock is meaningless for EXPECT FETCH in BUILDER {name:?}; change the EXPECT value instead"
        );
    }
    Ok(())
}

fn tag_reference(artifact_count: usize, artifact_name: &str, tag: &str) -> Result<String> {
    if artifact_count > 1 && tag.contains(':') {
        anyhow::bail!(
            "-t name:tag is ambiguous for a multi-artifact Cixfile; pass only the tag so each artifact is tagged as <block-name>:<tag>"
        );
    }
    Ok(if artifact_count == 1 && tag.contains(':') {
        tag.to_owned()
    } else {
        format!("{artifact_name}:{tag}")
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
    use super::{reject_expected_fetch_update, tag_reference};
    use crate::parse;

    #[test]
    fn multi_item_tags_are_item_names_and_reject_full_refs() {
        assert_eq!(tag_reference(2, "api", "v7").unwrap(), "api:v7".to_owned());
        let error = tag_reference(2, "api", "other:v7").unwrap_err().to_string();
        assert!(error.contains("multi-artifact"), "{error}");
        assert_eq!(
            tag_reference(1, "api", "other:v7").unwrap(),
            "other:v7".to_owned()
        );
    }

    #[test]
    fn update_lock_refuses_expected_fetches_in_both_forms() {
        let cixfile = parse(
            "FROM nixpkgs AS pkgs\nFETCH ingredient EXPECT sha256-one printf one\nBUILDER build\nPATH ${pkgs.bash}/bin\nFETCH EXPECT sha256-two printf two\nSERVICE app\nEXEC /bin/true\n",
        )
        .unwrap();
        let top = reject_expected_fetch_update(&cixfile, "ingredient")
            .unwrap_err()
            .to_string();
        assert!(top.contains("change the EXPECT value"), "{top}");
        let builder = reject_expected_fetch_update(&cixfile, "build")
            .unwrap_err()
            .to_string();
        assert!(builder.contains("change the EXPECT value"), "{builder}");
        assert!(reject_expected_fetch_update(&cixfile, "pkgs").is_ok());
    }
}
