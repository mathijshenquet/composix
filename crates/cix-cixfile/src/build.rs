use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{canonical, codegen::generate_nix_with_snapshots, ensure_lock, parse};
use cix_build::{
    execute, resolve_input_metadata, save_lock, ArtifactPin, ArtifactResolver, OutputReceipt,
};

pub trait ArtifactRegistry: ArtifactResolver {
    fn tag_artifact(&self, store_path: &str, reference: &str) -> Result<()>;
}

struct UnavailableRegistry;

impl ArtifactResolver for UnavailableRegistry {
    fn resolve_artifact(&self, reference: &str) -> Result<ArtifactPin> {
        anyhow::bail!(
            "resolving cix-item FROM ref {reference:?} requires an application artifact registry"
        )
    }
}

impl ArtifactRegistry for UnavailableRegistry {
    fn tag_artifact(&self, _store_path: &str, reference: &str) -> Result<()> {
        anyhow::bail!("tagging {reference:?} requires an application artifact registry")
    }
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub directory: PathBuf,
    pub update_lock: Option<String>,
    pub tag: Option<String>,
    pub cold: bool,
    pub allow_secret: bool,
    pub workspace_directory: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltItem {
    pub name: String,
    pub store_path: String,
    pub args: std::collections::BTreeMap<String, String>,
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
    build_with_registry(options, &UnavailableRegistry)
}

pub fn build_with_registry(
    options: &BuildOptions,
    registry: &dyn ArtifactRegistry,
) -> Result<Vec<BuiltItem>> {
    let tags = options.tag.iter().cloned().collect::<Vec<_>>();
    build_family_with_registry(options, &tags, None, None, registry)
}

pub fn build_with_stats(options: &BuildOptions) -> Result<(Vec<BuiltItem>, BuildStats)> {
    build_with_stats_and_registry(options, &UnavailableRegistry)
}

pub fn build_with_stats_and_registry(
    options: &BuildOptions,
    registry: &dyn ArtifactRegistry,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    let tags = options.tag.iter().cloned().collect::<Vec<_>>();
    build_family_with_stats_and_registry(options, &tags, None, None, registry)
}

pub fn build_family(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
) -> Result<Vec<BuiltItem>> {
    build_family_with_registry(
        options,
        tags,
        requested_namespace,
        selector,
        &UnavailableRegistry,
    )
}

pub fn build_family_with_registry(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
    registry: &dyn ArtifactRegistry,
) -> Result<Vec<BuiltItem>> {
    Ok(build_family_with_stats_and_registry(
        options,
        tags,
        requested_namespace,
        selector,
        registry,
    )?
    .0)
}

pub fn build_family_with_stats(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    build_family_with_stats_and_registry(
        options,
        tags,
        requested_namespace,
        selector,
        &UnavailableRegistry,
    )
}

pub fn build_family_with_stats_and_registry(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
    registry: &dyn ArtifactRegistry,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    build_family_with_stats_file_and_registry(
        options,
        tags,
        requested_namespace,
        selector,
        "Cixfile",
        registry,
    )
}

pub fn build_family_with_stats_file(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
    file_name: &str,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    build_family_with_stats_file_and_registry(
        options,
        tags,
        requested_namespace,
        selector,
        file_name,
        &UnavailableRegistry,
    )
}

pub fn build_family_with_stats_file_and_registry(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
    file_name: &str,
    registry: &dyn ArtifactRegistry,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    build_family_with_stats_file_and_registry_args(
        options,
        tags,
        requested_namespace,
        selector,
        file_name,
        &[],
        false,
        registry,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_family_with_stats_file_and_registry_args(
    options: &BuildOptions,
    tags: &[String],
    requested_namespace: Option<&str>,
    selector: Option<&str>,
    file_name: &str,
    arg_values: &[String],
    all_args: bool,
    registry: &dyn ArtifactRegistry,
) -> Result<(Vec<BuiltItem>, BuildStats)> {
    cix_common::reset_nix_subprocess_count();
    let directory = options
        .directory
        .canonicalize()
        .with_context(|| format!("resolving build directory {}", options.directory.display()))?;
    let cixfile_path = named_cixfile_path(&directory, file_name)?;
    let source = fs::read_to_string(&cixfile_path)
        .with_context(|| format!("reading {}", cixfile_path.display()))?;
    let declared = parse(&source).with_context(|| format!("parsing {}", cixfile_path.display()))?;
    let requested_args = parse_arg_values(arg_values)?;
    let cells = arg_cells(&declared, &requested_args, all_args)?;
    let multiple_cells = cells.len() > 1;
    if cells.len() > 1 && !tags.is_empty() {
        anyhow::bail!(
            "--all-args with -t needs an explicit per-cell tag mapping, which is not yet a CLI surface; build the cells, then tag selected outputs with `cix tag`; see docs/cixfile.md#build-args"
        )
    }
    let record_eval_plan = selector.is_none() && file_name == "Cixfile";
    let mut all_outputs = Vec::new();
    let mut all_steps = Vec::new();
    for selected_args in cells {
        let cell = (|| -> Result<(Vec<BuiltItem>, BuildStats)> {
            let cixfile = crate::parse_with_args(&source, &selected_args)
                .with_context(|| format!("selecting ARG cell for {}", cixfile_path.display()))?;
            let canonical_cixfile =
                canonical::serialize(&cixfile).context("serializing canonical Cixfile")?;
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
                anyhow::bail!(
                    "a tag names the whole family; do not combine a member selector with -t"
                )
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
                Some(name)
                    if cixfile.fetches.contains_key(name)
                        || cixfile.builders.contains_key(name) =>
                {
                    None
                }
                Some(name)
                    if cixfile
                        .inputs
                        .get(name)
                        .is_some_and(|input| input.is_local()) =>
                {
                    anyhow::bail!(
                        "FROM . AS {name} is the local build context and is not lock-pinned"
                    )
                }
                Some(name) => anyhow::bail!(
                    "--update-lock names no lock-bearing FROM, FETCH, or BUILDER binder {name:?}"
                ),
            };
            let lock_path = directory.join(format!("{file_name}.lock"));
            let mut lock = ensure_lock(registry, &lock_path, &cixfile.inputs, input_update)?;
            resolve_input_metadata(&mut cixfile, &lock)?;
            let expectations_validated =
                cix_build::validate_declared_expectations(&cixfile, &lock)?;
            let source_hash = build_cell_fingerprint(
                &directory,
                &lock,
                file_name,
                &canonical_cixfile,
                &selected_args,
            )?;
            if !options.cold
                && options.update_lock.is_none()
                && tags.is_empty()
                && expectations_validated
            {
                let cached = cixfile
                    .artifact_order
                    .iter()
                    .map(|name| {
                        lock.outputs
                            .get(&output_receipt_key(name, &selected_args))
                            .filter(|receipt| {
                                receipt.source_hash == source_hash
                                    && std::path::Path::new(&receipt.store_path).is_dir()
                            })
                            .map(|receipt| BuiltItem {
                                name: name.clone(),
                                store_path: receipt.store_path.clone(),
                                args: selected_args.clone(),
                            })
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(outputs) = cached {
                    for builder in &cixfile.builder_order {
                        eprintln!(
                            "BUILDER {builder} memo hit completed output (zero Nix subprocesses)"
                        );
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
            let execution = execute(
                &cixfile,
                &directory,
                &mut lock,
                &system,
                requested_update,
                options.cold,
                options.allow_secret,
                &options.workspace_directory,
                &crate::codegen::Codegen,
            );
            save_lock(&lock_path, &lock)?;
            let (snapshots, executed_steps) = execution?;
            if record_eval_plan {
                match cix_build::EvalPlan::from_cixfile(
                    &cixfile,
                    content_hash(&canonical_cixfile),
                    &lock,
                ) {
                    Ok(plan) => lock.eval_plan = Some(plan),
                    Err(error) => {
                        lock.eval_plan = None;
                        eprintln!("note: Cixfile.lock is not CIP-94-ready: {error:#}");
                    }
                }
                save_lock(&lock_path, &lock)?;
            }
            let mut outputs = Vec::new();
            for name in &cixfile.artifact_order {
                let expression = generate_nix_with_snapshots(
                    &cixfile, name, &directory, &lock, &system, &snapshots,
                )?;
                let realized = build_expression(&expression)?;
                let store_path = add_item_to_store(&realized, name)?;
                outputs.push(BuiltItem {
                    name: name.clone(),
                    store_path,
                    args: selected_args.clone(),
                });
            }
            let source_hash = build_cell_fingerprint(
                &directory,
                &lock,
                file_name,
                &canonical_cixfile,
                &selected_args,
            )?;
            for item in &outputs {
                for tag in tags {
                    let reference = tag_reference(namespace.as_deref(), &item.name, tag)?;
                    registry
                        .tag_artifact(&item.store_path, &reference)
                        .with_context(|| {
                            format!("tagging built member {:?} as {reference:?}", item.name)
                        })?;
                }
            }
            for item in &outputs {
                lock.outputs.insert(
                    output_receipt_key(&item.name, &selected_args),
                    OutputReceipt {
                        source_hash: source_hash.clone(),
                        store_path: item.store_path.clone(),
                        args: selected_args.clone(),
                    },
                );
            }
            save_lock(&lock_path, &lock)?;
            Ok((
                outputs,
                BuildStats {
                    steps: executed_steps
                        .into_iter()
                        .map(|step| StepStat {
                            name: step.name,
                            kind: step.kind,
                            status: if step.executed {
                                "executed"
                            } else {
                                "memo-hit"
                            },
                        })
                        .collect(),
                    nix_subprocesses: cix_common::nix_subprocess_count(),
                },
            ))
        })()?;
        all_outputs.extend(cell.0);
        all_steps.extend(cell.1.steps);
    }
    if multiple_cells {
        let lock_path = directory.join(format!("{file_name}.lock"));
        let mut lock: cix_build::LockFile = serde_json::from_slice(&fs::read(&lock_path)?)
            .with_context(|| format!("reading {}", lock_path.display()))?;
        for item in &all_outputs {
            let cixfile = crate::parse_with_args(&source, &item.args)
                .with_context(|| format!("selecting ARG cell for {}", cixfile_path.display()))?;
            let canonical_cixfile =
                canonical::serialize(&cixfile).context("serializing canonical Cixfile")?;
            let source_hash = build_cell_fingerprint(
                &directory,
                &lock,
                file_name,
                &canonical_cixfile,
                &item.args,
            )?;
            lock.outputs.insert(
                output_receipt_key(&item.name, &item.args),
                OutputReceipt {
                    source_hash,
                    store_path: item.store_path.clone(),
                    args: item.args.clone(),
                },
            );
        }
        save_lock(&lock_path, &lock)?;
    }
    Ok((
        all_outputs,
        BuildStats {
            steps: all_steps,
            nix_subprocesses: cix_common::nix_subprocess_count(),
        },
    ))
}

fn parse_arg_values(values: &[String]) -> Result<BTreeMap<String, String>> {
    let mut selected = BTreeMap::new();
    for value in values {
        let (name, value) = value.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "--arg expects NAME=value selecting a declared ARG cell; see docs/cixfile.md#build-args"
            )
        })?;
        if name.is_empty() || value.is_empty() {
            anyhow::bail!("--arg expects non-empty NAME=value; see docs/cixfile.md#build-args")
        }
        if selected.insert(name.to_owned(), value.to_owned()).is_some() {
            anyhow::bail!(
                "--arg {name} is repeated; select each declared ARG once; see docs/cixfile.md#build-args"
            )
        }
    }
    Ok(selected)
}

fn arg_cells(
    cixfile: &crate::Cixfile,
    requested: &BTreeMap<String, String>,
    all: bool,
) -> Result<Vec<BTreeMap<String, String>>> {
    if all && !requested.is_empty() {
        anyhow::bail!(
            "--all-args builds the complete declared matrix and cannot be combined with --arg; see docs/cixfile.md#build-args"
        )
    }
    for (name, value) in requested {
        let Some(argument) = cixfile.args.get(name) else {
            let matrix = cixfile
                .args
                .iter()
                .map(|(name, argument)| format!("{name}=[{}]", argument.values.join(", ")))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "--arg {name}={value} names no declared ARG; declared matrix: {}; see docs/cixfile.md#build-args",
                if matrix.is_empty() { "<none>" } else { &matrix }
            )
        };
        if !argument.values.contains(value) {
            anyhow::bail!(
                "--arg {name}={value} is outside the declared matrix [{}]; see docs/cixfile.md#build-args",
                argument.values.join(", ")
            )
        }
    }
    if !all {
        return Ok(vec![cixfile
            .args
            .iter()
            .map(|(name, argument)| {
                (
                    name.clone(),
                    requested
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| argument.values[0].clone()),
                )
            })
            .collect()]);
    }
    let mut cells = vec![BTreeMap::new()];
    for (name, argument) in &cixfile.args {
        cells = cells
            .into_iter()
            .flat_map(|cell| {
                argument.values.iter().map(move |value| {
                    let mut selected = cell.clone();
                    selected.insert(name.clone(), value.clone());
                    selected
                })
            })
            .collect();
    }
    Ok(cells)
}

fn output_receipt_key(name: &str, args: &BTreeMap<String, String>) -> String {
    if args.is_empty() {
        name.to_owned()
    } else {
        let cell = content_hash(&serde_json::to_vec(args).expect("ARG cell serialization"));
        format!("{name}@{}", &cell[..12])
    }
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

fn named_cixfile_path(directory: &std::path::Path, file_name: &str) -> Result<PathBuf> {
    let file = std::path::Path::new(file_name);
    if file.components().count() != 1
        || !matches!(
            file.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        anyhow::bail!("--file must name a Cixfile in the build directory, got {file_name:?}")
    }
    Ok(directory.join(file))
}

fn source_tree_hash(
    directory: &std::path::Path,
    file_name: &str,
    canonical_cixfile: &[u8],
) -> Result<String> {
    let mut digest = Sha256::new();
    hash_source_tree(
        directory,
        directory,
        file_name,
        canonical_cixfile,
        &mut digest,
    )?;
    let digest = digest.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn content_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn build_fingerprint(
    directory: &std::path::Path,
    lock: &cix_build::LockFile,
    file_name: &str,
    canonical_cixfile: &[u8],
) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(cix_build::BUILDER_FINGERPRINT.as_bytes());
    digest.update(source_tree_hash(directory, file_name, canonical_cixfile)?.as_bytes());
    // dev_envs is a derived builder cache; its semantic inputs are already keyed.
    digest.update(serde_json::to_vec(&(
        &lock.inputs,
        &lock.artifacts,
        &lock.fetches,
    ))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn build_cell_fingerprint(
    directory: &std::path::Path,
    lock: &cix_build::LockFile,
    file_name: &str,
    canonical_cixfile: &[u8],
    args: &BTreeMap<String, String>,
) -> Result<String> {
    let base = build_fingerprint(directory, lock, file_name, canonical_cixfile)?;
    if args.is_empty() {
        return Ok(base);
    }
    Ok(content_hash(&serde_json::to_vec(&(base, args))?))
}

fn hash_source_tree(
    root: &std::path::Path,
    path: &std::path::Path,
    file_name: &str,
    canonical_cixfile: &[u8],
    digest: &mut Sha256,
) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let relative = path.strip_prefix(root).unwrap_or(path);
    if is_cixfile_lock(relative) || relative.starts_with(".git") {
        return Ok(());
    }
    digest.update(relative.as_os_str().as_encoded_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(cix_build::nar_identity(path)?.as_bytes());
    } else if metadata.is_file() {
        if relative == std::path::Path::new(file_name) {
            digest.update(b"cixfile\0");
            digest.update([cix_build::executable_bit(&metadata)]);
            digest.update(canonical_cixfile);
        } else {
            digest.update(cix_build::nar_identity(path)?.as_bytes());
        }
    } else if metadata.is_dir() {
        digest.update(b"dir");
        let mut children = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            hash_source_tree(root, &child.path(), file_name, canonical_cixfile, digest)?;
        }
    }
    Ok(())
}

fn is_cixfile_lock(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "Cixfile.lock" || (name.starts_with("Cixfile.") && name.ends_with(".lock"))
        })
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
        build_family_with_stats_file, build_fingerprint, reject_expected_fetch_update,
        tag_namespace, tag_reference, validate_namespace, validate_tag, BuildOptions,
    };
    use crate::parse;
    use cix_build::{DevEnvironment, FetchPin, LockFile, OutputReceipt, ReadDependency, StepMemo};
    use std::collections::BTreeMap;

    fn canonical_cixfile(directory: &std::path::Path) -> Vec<u8> {
        crate::canonical::serialize(
            &parse(&std::fs::read_to_string(directory.join("Cixfile")).unwrap()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn output_receipts_do_not_change_the_build_fingerprint() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("Cixfile"),
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap();
        let mut lock = LockFile {
            inputs: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            fetches: BTreeMap::new(),
            memo: BTreeMap::new(),
            step_memo: BTreeMap::new(),
            dev_envs: BTreeMap::new(),
            builder_dev_envs: BTreeMap::new(),
            eval_plan: None,
            outputs: BTreeMap::new(),
        };
        let canonical = canonical_cixfile(directory.path());
        let before = build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();
        lock.outputs.insert(
            "app".into(),
            OutputReceipt {
                source_hash: "prior-run-hash".into(),
                store_path: "/nix/store/prior-run-output".into(),
                args: BTreeMap::new(),
            },
        );
        assert_eq!(
            before,
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap()
        );
    }

    #[test]
    fn build_fingerprint_uses_nar_invariant_source_tree_inputs() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("Cixfile"),
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("receipt.md"), "first receipt\n").unwrap();
        let mut lock = LockFile {
            inputs: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            fetches: BTreeMap::new(),
            memo: BTreeMap::new(),
            step_memo: BTreeMap::new(),
            dev_envs: BTreeMap::new(),
            builder_dev_envs: BTreeMap::new(),
            eval_plan: None,
            outputs: BTreeMap::new(),
        };
        let canonical = canonical_cixfile(directory.path());
        let initial = build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();

        std::fs::write(directory.path().join("receipt.md"), "second receipt\n").unwrap();
        let documentation_changed =
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();
        assert_ne!(initial, documentation_changed);

        std::fs::set_permissions(
            directory.path().join("receipt.md"),
            std::os::unix::fs::PermissionsExt::from_mode(0o600),
        )
        .unwrap();
        assert_eq!(
            documentation_changed,
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap()
        );
        std::fs::set_permissions(
            directory.path().join("receipt.md"),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        let executable_changed =
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();
        assert_ne!(documentation_changed, executable_changed);

        lock.step_memo.insert(
            "builder:build:0".into(),
            StepMemo {
                key: "trace-key".into(),
                reads: BTreeMap::from([(
                    "input".into(),
                    ReadDependency::File {
                        hash: "content-key".into(),
                        fingerprint: None,
                    },
                )]),
                output_snapshot: None,
                changes: BTreeMap::new(),
                output_hashes: BTreeMap::new(),
            },
        );
        assert_eq!(
            executable_changed,
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap()
        );

        lock.fetches.insert(
            "download".into(),
            FetchPin {
                nar_hash: String::new(),
                snapshot_nar_hash: String::new(),
                paths: BTreeMap::from([("archive".into(), "mode-dependent-hash".into())]),
                volatile: BTreeMap::new(),
            },
        );
        let fetch_pin_changed =
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();
        assert_ne!(executable_changed, fetch_pin_changed);

        lock.dev_envs.insert(
            "derived-cache-entry".into(),
            DevEnvironment {
                environment: BTreeMap::from([("PATH".into(), "/nix/store/example/bin".into())]),
            },
        );
        let derived_cache_rewritten =
            build_fingerprint(directory.path(), &lock, "Cixfile", &canonical).unwrap();
        assert_eq!(fetch_pin_changed, derived_cache_rewritten);
    }

    #[test]
    fn missing_named_cixfile_names_the_full_path() {
        let directory = tempfile::tempdir().unwrap();
        let options = BuildOptions {
            directory: directory.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
            workspace_directory: directory.path().join("workspaces"),
        };
        let error = build_family_with_stats_file(&options, &[], None, None, "Cixfile.dissolved")
            .unwrap_err()
            .to_string();
        assert!(error.contains("Cixfile.dissolved"), "{error}");
        assert!(
            error.contains(&directory.path().display().to_string()),
            "{error}"
        );
    }

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
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient printf one EXPECT sha256-one\nBUILDER build\nIMPORT ${pkgs.bash}\nFETCH printf two EXPECT sha256-two\nSERVICE app\nSTART /bin/true\n",
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
