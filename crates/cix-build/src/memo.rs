//! Build-step memo keys, validation, reduction, and constructive replay.
//!
//! `MemoEngine` owns memo policy and persisted memo state. The build conductor
//! supplies typed requests and acts on typed verdicts; filesystem changes are
//! performed only through `Workspace`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::fhs;
use crate::lock::builder_fetch_id;
use crate::trace;
use crate::workspace::{self, State, Workspace};
use crate::{
    BuildStep, Builder, ConsumedPath, Copy, LockFile, MemoEntry, ReadDependency, StepChange,
    StepMemo, TemplatePart,
};

const SANDBOX_SKELETON: &str = fhs::SKELETON_FINGERPRINT;
const CODEGEN_FINGERPRINT: &str = crate::BUILDER_FINGERPRINT;

#[derive(Serialize)]
pub(crate) struct StepKeyRequest<'a> {
    pub(crate) kind: &'a str,
    pub(crate) arguments: &'a str,
    pub(crate) offered_closure: &'a BTreeSet<String>,
    pub(crate) ordered_imports: &'a [String],
    pub(crate) predecessor: &'a str,
    pub(crate) declared_sources: &'a [String],
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) fetch_pin: Option<String>,
    pub(crate) universe_identities: &'a [String],
}

#[derive(Serialize)]
pub(crate) struct StepMemoKeyRequest<'a> {
    pub(crate) builder: &'a str,
    pub(crate) index: usize,
    pub(crate) kind: &'a str,
    pub(crate) directive: &'a str,
    pub(crate) arguments: &'a str,
    pub(crate) offered_closure: &'a BTreeSet<String>,
    pub(crate) ordered_imports: &'a [String],
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) universe_identities: &'a [String],
}

#[derive(Serialize)]
struct CopyKey<'a> {
    src: Vec<TemplateKeyPart<'a>>,
    dst: &'a str,
}

#[derive(Serialize)]
enum TemplateKeyPart<'a> {
    Literal(&'a str),
    Package {
        namespace: &'a str,
        attrpath: &'a str,
    },
    Binder(&'a str),
}

pub(crate) struct BuilderKeyRequest<'a> {
    pub(crate) builder_name: &'a str,
    pub(crate) builder: &'a Builder,
    pub(crate) commands: &'a [String],
    pub(crate) copies: &'a [String],
    pub(crate) ordered_imports: &'a [String],
    pub(crate) universe_identities: &'a [String],
    pub(crate) offered_closure: &'a BTreeSet<String>,
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) lock: &'a LockFile,
}

pub(crate) struct TopFetchKeyRequest<'a> {
    pub(crate) command: &'a str,
    pub(crate) offered_closure: &'a BTreeSet<String>,
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) pin: &'a str,
    pub(crate) universe_identities: &'a [String],
}

pub(crate) struct OutputMemoRequest<'a> {
    pub(crate) entry: Option<&'a MemoEntry>,
    pub(crate) needed: Vec<String>,
}

pub(crate) enum ChainKeysVerdict {
    Complete(Vec<String>),
    UnpinnedFetch,
}

pub(crate) enum OutputMemoVerdict {
    Hit { view: String },
    Miss,
}

pub(crate) struct StepReuseRequest<'a> {
    pub(crate) owner: &'a str,
    pub(crate) key: &'a str,
    pub(crate) memo: Option<&'a StepMemo>,
    pub(crate) is_fetch: bool,
    pub(crate) validate: bool,
    pub(crate) allow_reuse: bool,
}

pub(crate) enum StepReuseVerdict {
    Reused,
    Execute {
        known_reads: Option<BTreeMap<String, ReadDependency>>,
    },
}

pub(crate) struct ReductionRequest<'a> {
    pub(crate) previous: Option<&'a StepMemo>,
    pub(crate) before: &'a Path,
    pub(crate) is_fetch: bool,
    pub(crate) reads: BTreeMap<String, ReadDependency>,
    pub(crate) changes: BTreeMap<String, StepChange>,
    pub(crate) writes: &'a BTreeSet<String>,
    pub(crate) volatile: &'a BTreeSet<String>,
}

pub(crate) struct TopReductionRequest<'a> {
    pub(crate) before: &'a Path,
    pub(crate) workspace: &'a Path,
    pub(crate) reads: BTreeMap<String, ReadDependency>,
    pub(crate) changes: BTreeMap<String, StepChange>,
    pub(crate) writes: &'a BTreeSet<String>,
    pub(crate) volatile: &'a BTreeSet<String>,
}

pub(crate) struct ColdReadRequest<'a> {
    pub(crate) memo: &'a StepMemo,
    pub(crate) workspace: &'a Path,
    pub(crate) line: usize,
    pub(crate) source: &'a str,
}

pub(crate) struct ColdOutputRequest<'a> {
    pub(crate) warm: Option<&'a MemoEntry>,
    pub(crate) cold: &'a BTreeMap<String, ConsumedPath>,
    pub(crate) needed: &'a BTreeMap<String, NeededPath>,
}

pub(crate) struct ColdReadComparisonRequest<'a> {
    pub(crate) memo: &'a StepMemo,
    pub(crate) cold: &'a BTreeMap<String, ReadDependency>,
    pub(crate) line: usize,
    pub(crate) source: &'a str,
}

pub(crate) struct ReducedMemo {
    pub(crate) reads: BTreeMap<String, ReadDependency>,
    pub(crate) changes: BTreeMap<String, StepChange>,
    pub(crate) output_hashes: BTreeMap<String, crate::OutputHash>,
    pub(crate) output_fingerprints: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutedStep {
    pub name: String,
    pub kind: String,
    pub executed: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct NeededPath {
    pub(crate) attributions: Vec<Attribution>,
}

#[derive(Clone, Debug)]
pub(crate) struct Attribution {
    pub(crate) binder: String,
    pub(crate) path: String,
    pub(crate) line: usize,
}

pub(crate) struct MemoEngine<'workspace> {
    workspace: &'workspace Workspace,
    state: State,
}

impl<'workspace> MemoEngine<'workspace> {
    pub(crate) fn builder_keys(request: BuilderKeyRequest<'_>) -> Result<ChainKeysVerdict> {
        builder_chain_keys(request)
    }

    pub(crate) fn top_fetch_key(request: TopFetchKeyRequest<'_>) -> Result<String> {
        top_fetch_chain_key(
            request.command,
            request.offered_closure,
            request.environment,
            request.pin,
            request.universe_identities,
        )
    }

    pub(crate) fn trace_key(request: StepMemoKeyRequest<'_>) -> Result<String> {
        step_memo_key(request)
    }

    pub(crate) fn output(request: OutputMemoRequest<'_>) -> Result<OutputMemoVerdict> {
        output_memo(request.entry, request.needed.into_iter())
    }

    pub(crate) fn reduce_top(request: TopReductionRequest<'_>) -> Result<ReducedMemo> {
        let TopReductionRequest {
            before,
            workspace,
            mut reads,
            mut changes,
            writes,
            volatile,
        } = request;
        retain_nonvolatile_reads(&mut reads, volatile);
        trace::record_workspace_fingerprints(workspace, &mut reads, writes)?;
        trace::aggregate_full_read_subtrees(before, &mut reads)?;
        changes.retain(|path, _| !path_overlaps_any(path, volatile));
        trace::aggregate_full_change_subtrees(before, workspace, &mut changes)?;
        let output_hashes = workspace::memo_output_hashes(workspace, &changes)?;
        Ok(ReducedMemo {
            reads,
            changes,
            output_hashes,
            output_fingerprints: BTreeMap::new(),
        })
    }

    pub(crate) fn audit_cold(request: ColdReadRequest<'_>) -> Result<()> {
        verify_cold_read_set(
            request.memo,
            request.workspace,
            request.line,
            request.source,
        )
    }

    pub(crate) fn compare_cold_outputs(request: ColdOutputRequest<'_>) -> Result<()> {
        compare_cold_paths(request.warm, request.cold, request.needed)
    }

    pub(crate) fn compare_cold_reads(request: ColdReadComparisonRequest<'_>) -> Result<()> {
        compare_cold_read_sets(request.memo, request.cold, request.line, request.source)
    }

    pub(crate) fn entry(paths: BTreeMap<String, ConsumedPath>) -> MemoEntry {
        memo_entry(paths)
    }

    pub(crate) fn step_result(
        builder: &str,
        index: usize,
        kind: &str,
        executed: bool,
    ) -> ExecutedStep {
        executed_step(builder, index, kind, executed)
    }

    pub(crate) fn builder_results(
        builder_name: &str,
        builder: &Builder,
        executed: bool,
    ) -> Vec<ExecutedStep> {
        builder_step_results(builder_name, builder, executed)
    }

    pub(crate) fn new(workspace: &'workspace Workspace) -> Self {
        Self {
            workspace,
            state: workspace.load_state(),
        }
    }

    pub(crate) fn rerun_from(
        &self,
        existing_keys: Option<&[String]>,
        cold: bool,
        update_fetch_pins: bool,
    ) -> usize {
        let first_changed = existing_keys.map_or(0, |keys| {
            keys.iter()
                .zip(&self.state.step_keys)
                .take_while(|(current, prior)| current == prior)
                .count()
        });
        let warm_rerun_from = existing_keys
            .filter(|keys| *keys != self.state.step_keys.as_slice())
            .map_or(0, |_| first_changed);
        if cold || update_fetch_pins {
            0
        } else {
            warm_rerun_from
        }
    }

    pub(crate) fn try_reuse(&mut self, request: StepReuseRequest<'_>) -> Result<StepReuseVerdict> {
        let Some(memo) = request.memo.filter(|_| request.validate) else {
            return Ok(StepReuseVerdict::Execute { known_reads: None });
        };
        let fingerprints = request
            .is_fetch
            .then(|| self.state.memo_output_fingerprints.get(request.owner))
            .flatten()
            .filter(|_| self.state.materialized_memos.get(request.owner) == Some(&memo.key));
        let (matches, current) = self.validate(memo, request.is_fetch, fingerprints)?;
        if !matches || !request.allow_reuse {
            return Ok(StepReuseVerdict::Execute {
                known_reads: Some(current),
            });
        }
        if request.is_fetch {
            self.workspace.apply_memo(memo, fingerprints)?;
            self.state
                .materialized_memos
                .insert(request.owner.to_owned(), request.key.to_owned());
            self.state.memo_output_fingerprints.insert(
                request.owner.to_owned(),
                self.workspace.output_fingerprints(&memo.changes)?,
            );
        } else if self.state.materialized_memos.get(request.owner) == Some(&memo.key) {
            crate::cix_timing!("CIX timing memo-apply skipped=workspace-already-materialized");
        } else {
            self.workspace.apply_memo(memo, None)?;
            self.state
                .materialized_memos
                .insert(request.owner.to_owned(), request.key.to_owned());
        }
        Ok(StepReuseVerdict::Reused)
    }

    fn validate(
        &self,
        memo: &StepMemo,
        allow_fetch_self_reads: bool,
        output_fingerprints: Option<&BTreeMap<String, String>>,
    ) -> Result<(bool, BTreeMap<String, ReadDependency>)> {
        let validation_started = Instant::now();
        let replayable = self.workspace.memo_replayable(memo)?;
        let (current, metrics) =
            trace::current_dependencies_with_metrics(self.workspace.path(), &memo.reads)?;
        crate::cix_timing!(
            "CIX timing memo-validation rehashed_files={} rehashed_bytes={}",
            metrics.rehashed_files,
            metrics.rehashed_bytes
        );
        let self_matches = if current == memo.reads || !allow_fetch_self_reads || !replayable {
            false
        } else {
            let self_validation_started = Instant::now();
            let matches =
                self.workspace
                    .memo_self_state_matches(memo, &current, output_fingerprints)?;
            crate::cix_timing!(
                "CIX timing memo-self-validation wall_ms={}",
                self_validation_started.elapsed().as_millis()
            );
            matches
        };
        let matches = current == memo.reads || self_matches;
        crate::cix_timing!(
            "CIX timing memo-validation total_wall_ms={}",
            validation_started.elapsed().as_millis()
        );
        Ok((replayable && matches, current))
    }

    pub(crate) fn replay_cold(&self, memo: &StepMemo) -> Result<()> {
        self.workspace.apply_memo(memo, None)
    }

    pub(crate) fn revert(&self, memo: &StepMemo) -> Result<()> {
        self.workspace.revert_memo(memo)
    }

    pub(crate) fn audit_cold_reads(
        &self,
        memo: &StepMemo,
        line: usize,
        source: &str,
    ) -> Result<()> {
        verify_cold_read_set(memo, self.workspace.path(), line, source)
    }

    pub(crate) fn reduce(&self, request: ReductionRequest<'_>) -> Result<ReducedMemo> {
        let ReductionRequest {
            previous,
            before,
            is_fetch,
            mut reads,
            mut changes,
            writes,
            volatile,
        } = request;
        if let Some(previous) = previous {
            retain_replay_roots(previous, self.workspace.path(), &mut changes)?;
        }
        if is_fetch {
            retain_fetch_output_roots(before, self.workspace.path(), &mut changes)?;
        }
        retain_nonvolatile_reads(&mut reads, volatile);
        trace::record_workspace_fingerprints(self.workspace.path(), &mut reads, writes)?;
        trace::aggregate_full_read_subtrees(before, &mut reads)?;
        changes.retain(|path, _| !path_overlaps_any(path, volatile));
        trace::aggregate_full_change_subtrees(before, self.workspace.path(), &mut changes)?;
        let output_hashes = if is_fetch {
            self.workspace.output_hashes(&changes)?
        } else {
            BTreeMap::new()
        };
        let output_fingerprints = if is_fetch {
            self.workspace.output_fingerprints(&changes)?
        } else {
            BTreeMap::new()
        };
        Ok(ReducedMemo {
            reads,
            changes,
            output_hashes,
            output_fingerprints,
        })
    }

    pub(crate) fn record_materialized(
        &mut self,
        owner: String,
        key: String,
        output_fingerprints: Option<BTreeMap<String, String>>,
    ) {
        self.state.materialized_memos.insert(owner.clone(), key);
        if let Some(output_fingerprints) = output_fingerprints {
            self.state
                .memo_output_fingerprints
                .insert(owner, output_fingerprints);
        }
    }

    pub(crate) fn invalidate_fetch_outputs(
        &mut self,
        lock: &LockFile,
        changes: &BTreeMap<String, StepChange>,
    ) {
        self.state.memo_output_fingerprints.retain(|owner, _| {
            let Some(memo) = lock.step_memo.get(owner) else {
                return false;
            };
            self.state.materialized_memos.get(owner) == Some(&memo.key)
                && !memo.changes.keys().any(|output| {
                    changes.keys().any(|changed| {
                        same_or_descendant(output, changed) || same_or_descendant(changed, output)
                    })
                })
        });
    }

    pub(crate) fn finish(mut self, step_keys: Vec<String>, lock: &LockFile) -> Result<()> {
        for (owner, output_fingerprints) in &mut self.state.memo_output_fingerprints {
            let Some(memo) = lock.step_memo.get(owner) else {
                continue;
            };
            if self.state.materialized_memos.get(owner) == Some(&memo.key) {
                *output_fingerprints = self.workspace.output_fingerprints(&memo.changes)?;
            }
        }
        self.state.step_keys = step_keys;
        self.workspace.save_state(&self.state)
    }
}

fn output_memo(
    entry: Option<&MemoEntry>,
    needed: impl Iterator<Item = String>,
) -> Result<OutputMemoVerdict> {
    let needed = needed.collect::<Vec<_>>();
    if !workspace::memo_has_paths(entry, needed.into_iter())? {
        return Ok(OutputMemoVerdict::Miss);
    }
    let entry = entry.expect("validated memo exists");
    Ok(OutputMemoVerdict::Hit {
        view: workspace::materialize_view(&entry.paths)?,
    })
}

fn builder_chain_keys(request: BuilderKeyRequest<'_>) -> Result<ChainKeysVerdict> {
    let mut environment = request.environment.clone();
    let mut predecessor = hex_hash(format!("BUILDER\0{}", request.builder_name).as_bytes());
    let mut keys = Vec::with_capacity(request.builder.steps.len());
    let mut command_index = 0;
    let mut copy_index = 0;
    for (index, step) in request.builder.steps.iter().enumerate() {
        let (kind, arguments, sources, fetch_pin) = match step {
            BuildStep::Env { name, value, .. } => {
                let value = value
                    .literal_value()
                    .context("builder ENV metadata was not resolved")?;
                environment.insert(name.clone(), value.clone());
                ("ENV", format!("{name}={value}"), Vec::new(), None)
            }
            BuildStep::Copy(copy) => {
                let source = &request.copies[copy_index];
                copy_index += 1;
                (
                    "COPY",
                    copy_key_arguments(copy)?,
                    vec![workspace::nar_hash(Path::new(source))
                        .with_context(|| format!("hashing declared COPY source {source}"))?],
                    None,
                )
            }
            BuildStep::Fetch { .. } => {
                let command = &request.commands[command_index];
                command_index += 1;
                let id = builder_fetch_id(request.builder_name, index, command);
                let Some(pin) = request.lock.fetches.get(&id) else {
                    return Ok(ChainKeysVerdict::UnpinnedFetch);
                };
                ("FETCH", command.clone(), Vec::new(), Some(pin.key()))
            }
            BuildStep::Run { .. } => {
                let command = &request.commands[command_index];
                command_index += 1;
                ("RUN", command.clone(), Vec::new(), None)
            }
        };
        predecessor = step_key(StepKeyRequest {
            kind,
            arguments: &arguments,
            offered_closure: request.offered_closure,
            ordered_imports: request.ordered_imports,
            predecessor: &predecessor,
            declared_sources: &sources,
            environment: &environment,
            fetch_pin,
            universe_identities: request.universe_identities,
        })?;
        keys.push(predecessor.clone());
    }
    Ok(ChainKeysVerdict::Complete(keys))
}

fn top_fetch_chain_key(
    command: &str,
    offered_closure: &BTreeSet<String>,
    environment: &BTreeMap<String, String>,
    pin: &str,
    universe_identities: &[String],
) -> Result<String> {
    step_key(StepKeyRequest {
        kind: "FETCH",
        arguments: command,
        offered_closure,
        ordered_imports: &[],
        predecessor: &hex_hash(b"TOP-LEVEL-FETCH"),
        declared_sources: &[],
        environment,
        fetch_pin: Some(pin.to_owned()),
        universe_identities,
    })
}

fn step_key(request: StepKeyRequest<'_>) -> Result<String> {
    Ok(hex_hash(&serde_json::to_vec(&(
        CODEGEN_FINGERPRINT,
        SANDBOX_SKELETON,
        request,
    ))?))
}

fn step_memo_key(request: StepMemoKeyRequest<'_>) -> Result<String> {
    Ok(hex_hash(&serde_json::to_vec(&(
        CODEGEN_FINGERPRINT,
        SANDBOX_SKELETON,
        request,
    ))?))
}

fn copy_key_arguments(copy: &Copy) -> Result<String> {
    let src = copy
        .src
        .parts
        .iter()
        .map(|part| match part {
            TemplatePart::Literal(value) => TemplateKeyPart::Literal(value),
            TemplatePart::Package {
                namespace,
                attrpath,
                ..
            } => TemplateKeyPart::Package {
                namespace,
                attrpath,
            },
            TemplatePart::Binder { name, .. } => TemplateKeyPart::Binder(name),
            TemplatePart::InputMetadata {
                namespace,
                attribute,
                ..
            } => unreachable!("unresolved FROM metadata {namespace}.{attribute}"),
        })
        .collect();
    Ok(serde_json::to_string(&CopyKey {
        src,
        dst: &copy.dst,
    })?)
}

fn retain_nonvolatile_reads(
    reads: &mut BTreeMap<String, ReadDependency>,
    volatile: &BTreeSet<String>,
) {
    reads.retain(|path, _| {
        !volatile.iter().any(|volatile_path| {
            volatile_path == path
                || volatile_path
                    .strip_prefix(path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    });
}

fn retain_replay_roots(
    previous: &StepMemo,
    workspace: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    for (root, previous_change) in &previous.changes {
        let path = workspace.join(root);
        match (previous_change, fs::symlink_metadata(&path)) {
            (change @ (StepChange::Present | StepChange::Subtree { .. }), Ok(_)) => {
                changes.retain(|candidate, _| !same_or_descendant(candidate, root));
                changes.insert(root.clone(), change.clone());
            }
            (StepChange::Present | StepChange::Subtree { .. } | StepChange::Absent, Err(error))
                if error.kind() == io::ErrorKind::NotFound =>
            {
                changes.insert(root.clone(), StepChange::Absent);
            }
            (StepChange::Directory { mode }, Ok(metadata)) if metadata.is_dir() => {
                changes.insert(root.clone(), StepChange::Directory { mode: *mode });
            }
            (_, Ok(_)) => {}
            (_, Err(error)) => return Err(error.into()),
        }
    }
    Ok(())
}

fn retain_fetch_output_roots(
    before: &Path,
    workspace: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    let mut roots = BTreeMap::<String, usize>::new();
    for path in changes.keys() {
        if let Some(root) = path.split('/').next() {
            *roots.entry(root.to_owned()).or_default() += 1;
        }
    }
    for (root, changed_descendants) in roots {
        if changed_descendants < 2 {
            continue;
        }
        let before_root = before.join(&root);
        let workspace_root = workspace.join(&root);
        let absent_before = match fs::symlink_metadata(&before_root) {
            Ok(_) => false,
            Err(error) if error.kind() == io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };
        let present_after = match fs::symlink_metadata(&workspace_root) {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !absent_before || !present_after {
            continue;
        }
        changes.retain(|path, _| !same_or_descendant(path, &root));
        changes.insert(root, StepChange::Present);
    }
    Ok(())
}

fn path_overlaps_any(path: &str, paths: &BTreeSet<String>) -> bool {
    paths.iter().any(|other| {
        path == other
            || path
                .strip_prefix(other)
                .is_some_and(|suffix| suffix.starts_with('/'))
            || other
                .strip_prefix(path)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn verify_cold_read_set(
    memo: &StepMemo,
    workspace: &Path,
    line: usize,
    source: &str,
) -> Result<()> {
    let current = trace::current_dependencies(workspace, &memo.reads)?;
    compare_cold_read_sets(memo, &current, line, source)
}

fn compare_cold_read_sets(
    memo: &StepMemo,
    cold: &BTreeMap<String, ReadDependency>,
    line: usize,
    source: &str,
) -> Result<()> {
    if memo.reads == *cold {
        return Ok(());
    }
    let path = memo
        .reads
        .keys()
        .chain(cold.keys())
        .find(|path| memo.reads.get(*path) != cold.get(*path))
        .map(String::as_str)
        .unwrap_or("<unknown>");
    bail!(
        "line {line}: recorded read set differs between warm and cold at {path:?} (warm {:?}, cold {:?})\n  | {source:?}",
        memo.reads.get(path),
        cold.get(path)
    )
}

fn executed_step(builder: &str, index: usize, kind: &str, executed: bool) -> ExecutedStep {
    ExecutedStep {
        name: format!("{builder}:{}", index + 1),
        kind: kind.into(),
        executed,
    }
}

fn builder_step_results(
    builder_name: &str,
    builder: &Builder,
    executed: bool,
) -> Vec<ExecutedStep> {
    builder
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let kind = match step {
                BuildStep::Env { .. } => "ENV",
                BuildStep::Copy(_) => "COPY",
                BuildStep::Fetch { .. } => "FETCH",
                BuildStep::Run { .. } => "RUN",
            };
            executed_step(builder_name, index, kind, executed)
        })
        .collect()
}

fn compare_cold_paths(
    warm: Option<&MemoEntry>,
    cold: &BTreeMap<String, ConsumedPath>,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<()> {
    let Some(warm) = warm else {
        return Ok(());
    };
    for (path, cold_path) in cold {
        let differs = warm
            .paths
            .get(path)
            .is_none_or(|warm_path| warm_path.nar_hash != cold_path.nar_hash);
        if !differs {
            continue;
        }
        if let Some(attribution) = needed[path].attributions.first() {
            let suffix = if attribution.path == "." {
                String::new()
            } else {
                format!("/{}", attribution.path)
            };
            bail!(
                "COPY ${{{}}}{suffix} (line {}) differs between warm and cold",
                attribution.binder,
                attribution.line
            );
        }
        bail!("consumed path {path:?} differs between warm and cold");
    }
    Ok(())
}

fn memo_entry(paths: BTreeMap<String, ConsumedPath>) -> MemoEntry {
    MemoEntry { paths }
}

fn same_or_descendant(candidate: &str, root: &str) -> bool {
    candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
pub(crate) fn test_step_key(request: StepKeyRequest<'_>) -> Result<String> {
    step_key(request)
}

#[cfg(test)]
pub(crate) fn test_step_memo_key(request: StepMemoKeyRequest<'_>) -> Result<String> {
    step_memo_key(request)
}

#[cfg(test)]
pub(crate) fn test_top_fetch_chain_key(
    command: &str,
    offered_closure: &BTreeSet<String>,
    environment: &BTreeMap<String, String>,
    pin: &str,
    universe_identities: &[String],
) -> Result<String> {
    top_fetch_chain_key(
        command,
        offered_closure,
        environment,
        pin,
        universe_identities,
    )
}

#[cfg(test)]
pub(crate) fn test_copy_key_arguments(copy: &Copy) -> Result<String> {
    copy_key_arguments(copy)
}

#[cfg(test)]
pub(crate) fn test_memo_entry(paths: BTreeMap<String, ConsumedPath>) -> MemoEntry {
    memo_entry(paths)
}

#[cfg(test)]
pub(crate) fn test_compare_cold_paths(
    warm: Option<&MemoEntry>,
    cold: &BTreeMap<String, ConsumedPath>,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<()> {
    compare_cold_paths(warm, cold, needed)
}

#[cfg(test)]
pub(crate) fn test_verify_cold_read_set(
    memo: &StepMemo,
    workspace: &Path,
    line: usize,
    source: &str,
) -> Result<()> {
    verify_cold_read_set(memo, workspace, line, source)
}

#[cfg(test)]
pub(crate) fn test_retain_fetch_output_roots(
    before: &Path,
    workspace: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    retain_fetch_output_roots(before, workspace, changes)
}

#[cfg(test)]
pub(crate) fn test_validate_step_memo(
    memo: &StepMemo,
    workspace: &Path,
    allow_fetch_self_reads: bool,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<(bool, BTreeMap<String, ReadDependency>)> {
    let workspace = Workspace::borrowed_for_test(workspace);
    MemoEngine::new(&workspace).validate(memo, allow_fetch_self_reads, output_fingerprints)
}
