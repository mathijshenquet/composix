use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ensure_lock, parse};
use cix_build::{
    execute, generate_nix_with_snapshots, resolve_input_metadata, save_lock, OutputReceipt,
};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub directory: PathBuf,
    pub update_lock: Option<String>,
    pub tag: Option<String>,
    pub cold: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltItem {
    pub name: String,
    pub store_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildStats {
    pub steps: Vec<StepStat>,
    #[serde(rename = "nixSubprocesses")]
    pub nix_subprocesses: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StepStat {
    pub name: String,
    pub kind: String,
    pub status: &'static str,
}

pub fn build(options: &BuildOptions) -> Result<Vec<BuiltItem>> {
    let tags = options.tag.iter().cloned().collect::<Vec<_>>();
    build_family(options, &tags, None, None)
}

pub fn build_with_stats(options: &BuildOptions) -> Result<(Vec<BuiltItem>, BuildStats)> {
    let tags = options.tag.iter().cloned().collect::<Vec<_>>();
    build_family_with_stats(options, &tags, None, None)
}

pub fn build_family(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
) -> Result<Vec<BuiltItem>> {
    Ok(build_family_with_stats(options, tags, requested_namespace, selector)?.0)
}

pub fn build_family_with_stats(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    cix_common::reset_nix_subprocess_count();
    let directory = options
        .directory
        .canonicalize()
        .with_context(|| format!("resolving build directory {}", options.directory.display()))?;
    let cixfile_path = directory.join("Cixfile");
    let source = fs::read_to_string(&cixfile_path)
        .with_context(|| format!("reading {}", cixfile_path.display()))?;
    let cixfile = parse(&source).with_context(|| format!("parsing {}", cixfile_path.display()))?;
    let mut cixfile = match selector {
        Some(member) => cixfile.backward_slice(member).with_context(|| {
            format!(
                "unknown Cixfile member {member:?}; available members: {}",
                cixfile.artifact_order.join(", ")
            )
        })?,
        None => cixfile,
    };
    if selector.is_some() && !tags.is_empty() {
        anyhow::bail!("a tag names the whole family; do not combine a member selector with -t")
    }
    if requested_namespace.is_some() && tags.is_empty() {
        anyhow::bail!("--namespace is only meaningful with -t")
    }
    let namespace = tag_namespace(&cixfile, requested_namespace, tags)?;
    if let Some(requested) = options.update_lock.as_deref() {
        reject_expected_fetch_update(&cixfile, requested)?;
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
    resolve_input_metadata(&mut cixfile, &lock)?;
    let source_hash = build_fingerprint(&directory, &lock)?;
    if !options.cold && options.update_lock.is_none() && tags.is_empty() {
        let cached = cixfile
            .artifact_order
            .iter()
            .map(|name| {
                lock.outputs
                    .get(name)
                    .filter(|receipt| {
                        receipt.source_hash == source_hash
                            && std::path::Path::new(&receipt.store_path).is_dir()
                    })
                    .map(|receipt| BuiltItem {
                        name: name.clone(),
                        store_path: receipt.store_path.clone(),
                    })
            })
            .collect::<Option<Vec<_>>>();
        if let Some(outputs) = cached {
            for builder in &cixfile.builder_order {
                eprintln!("BUILDER {builder} memo hit completed output (zero Nix subprocesses)");
            }
            return Ok((
                outputs,
                BuildStats {
                    steps: step_stats(&cixfile, "memo-hit"),
                    nix_subprocesses: cix_common::nix_subprocess_count(),
                },
            ));
        }
    }
    let system = cix_common::current_system()?;
    let snapshots = execute(
        &cixfile,
        &directory,
        &mut lock,
        &system,
        requested_update,
        options.cold,
    );
    save_lock(&lock_path, &lock)?;
    let snapshots = snapshots?;
    let mut outputs = Vec::new();
    for name in &cixfile.artifact_order {
        let expression =
            generate_nix_with_snapshots(&cixfile, name, &directory, &lock, &system, &snapshots)?;
        let realized = build_expression(&expression)?;
        let store_path = add_item_to_store(&realized, name)?;
        outputs.push(BuiltItem {
            name: name.clone(),
            store_path,
        });
    }
    let source_hash = build_fingerprint(&directory, &lock)?;
    for item in &outputs {
        for tag in tags {
            let reference = tag_reference(namespace.as_deref(), &item.name, tag)?;
            cix_index::tag(&item.store_path, &reference, None).with_context(|| {
                format!("tagging built member {:?} as {reference:?}", item.name)
            })?;
        }
    }
    for item in &outputs {
        lock.outputs.insert(
            item.name.clone(),
            OutputReceipt {
                source_hash: source_hash.clone(),
                store_path: item.store_path.clone(),
            },
        );
    }
    save_lock(&lock_path, &lock)?;
    Ok((
        outputs,
        BuildStats {
            steps: step_stats(&cixfile, "executed"),
            nix_subprocesses: cix_common::nix_subprocess_count(),
        },
    ))
}

fn step_stats(cixfile: &crate::Cixfile, status: &'static str) -> Vec<StepStat> {
    let mut stats = Vec::new();
    for name in &cixfile.fetch_order {
        stats.push(StepStat {
            name: name.clone(),
            kind: "FETCH".into(),
            status,
        });
    }
    for builder_name in &cixfile.builder_order {
        for (index, step) in cixfile.builders[builder_name].steps.iter().enumerate() {
            let kind = match step {
                cix_build::BuildStep::Env { .. } => "ENV",
                cix_build::BuildStep::Copy(_) => "COPY",
                cix_build::BuildStep::Fetch { .. } => "FETCH",
                cix_build::BuildStep::Run { .. } => "RUN",
            };
            stats.push(StepStat {
                name: format!("{builder_name}:{}", index + 1),
                kind: kind.into(),
                status,
            });
        }
    }
    stats
}

fn source_tree_hash(directory: &std::path::Path) -> Result<String> {
    let mut digest = Sha256::new();
    hash_source_tree(directory, directory, &mut digest)?;
    let digest = digest.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn build_fingerprint(directory: &std::path::Path, lock: &cix_build::LockFile) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(source_tree_hash(directory)?.as_bytes());
    digest.update(serde_json::to_vec(&(
        &lock.inputs,
        &lock.artifacts,
        &lock.fetches,
        &lock.dev_envs,
    ))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn hash_source_tree(
    root: &std::path::Path,
    path: &std::path::Path,
    digest: &mut Sha256,
) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative == std::path::Path::new("Cixfile.lock") || relative.starts_with(".git") {
        return Ok(());
    }
    digest.update(relative.as_os_str().as_encoded_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"link");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if metadata.is_file() {
        digest.update(b"file");
        if relative == std::path::Path::new("Cixfile") {
            digest.update(crate::fmt::format(&fs::read_to_string(path)?)?.as_bytes());
        } else {
            let mut file = fs::File::open(path)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            digest.update(bytes);
        }
    } else if metadata.is_dir() {
        digest.update(b"dir");
        let mut children = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            hash_source_tree(root, &child.path(), digest)?;
        }
    }
    Ok(())
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

fn tag_namespace(
    cixfile: &crate::Cixfile,
    namespace: Option<&str>,
    tags: &[String],
) -> Result<Option<String>> {
    if tags.is_empty() {
        return Ok(None);
    }
    for tag in tags {
        validate_tag(tag)?;
    }
    match (cixfile.artifacts.len(), namespace) {
        (count, None) if count > 1 => anyhow::bail!(
            "a multi-artifact Cixfile needs --namespace when tagging; bare sibling names must never enter the index"
        ),
        (_, Some(namespace)) => {
            validate_namespace(namespace)?;
            Ok(Some(namespace.to_owned()))
        }
        (_, None) => Ok(None),
    }
}

fn validate_tag(tag: &str) -> Result<()> {
    if tag.contains(':') || tag.contains('/') {
        anyhow::bail!(
            "member names live in the Cixfile (SERVICE); the family name is --namespace; -t takes only tags"
        );
    }
    cix_common::Ref::parse(&format!("member:{tag}"))?;
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if namespace.contains("://") {
        anyhow::bail!("--namespace must be schemeless: scheme is transport, not identity")
    }
    let reference = format!("{namespace}/member:tag");
    let parsed = cix_common::Ref::parse(&reference)
        .with_context(|| format!("invalid --namespace {namespace:?}"))?;
    if namespace.contains('/') && parsed.root_url.is_none() {
        anyhow::bail!(
            "invalid --namespace {namespace:?}; use one family segment, optionally qualified as host/family"
        )
    }
    if parsed.root_url.is_some() && parsed.name.split('/').count() != 2 {
        anyhow::bail!(
            "invalid --namespace {namespace:?}; a qualified namespace needs both host and family"
        )
    }
    Ok(())
}

fn tag_reference(namespace: Option<&str>, member: &str, tag: &str) -> Result<String> {
    let reference = match namespace {
        Some(namespace) => format!("{namespace}/{member}:{tag}"),
        None => format!("{member}:{tag}"),
    };
    cix_common::Ref::parse(&reference)?;
    Ok(reference)
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
    use super::{
        reject_expected_fetch_update, tag_namespace, tag_reference, validate_namespace,
        validate_tag,
    };
    use crate::parse;

    #[test]
    fn tags_are_tag_only_and_namespaces_supply_family_names() {
        let multi = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE api\nSTART /bin/true\nSERVICE worker\nSTART /bin/true\n",
        )
        .unwrap();
        let error = tag_namespace(&multi, None, &["v7".into()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("--namespace"), "{error}");
        assert_eq!(
            tag_namespace(&multi, Some("family"), &["v7".into()]).unwrap(),
            Some("family".into())
        );
        assert_eq!(
            tag_reference(Some("family"), "api", "v7").unwrap(),
            "family/api:v7"
        );
        let error = validate_tag("family:v7").unwrap_err().to_string();
        assert!(error.contains("member names live"), "{error}");
        let error = validate_namespace("https://example.com/family")
            .unwrap_err()
            .to_string();
        assert!(error.contains("scheme is transport"), "{error}");
    }

    #[test]
    fn update_lock_refuses_expected_fetches_in_both_forms() {
        let cixfile = parse(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT sha256-one printf one\nBUILDER build\nIMPORT ${pkgs.bash}\nFETCH EXPECT sha256-two printf two\nSERVICE app\nSTART /bin/true\n",
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
