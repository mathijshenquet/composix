use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::evaluation::{
    BuilderContextRequest, DevEnvironmentRequest, EvaluationCodegen, FetchContextRequest,
    NixEvaluation,
};
use crate::evidence;
use crate::fetch::HostCredentials;
use crate::fetch_state::{FetchState, PinRefreshRequest};
use crate::lock::{builder_fetch_id, resolved_statement_id};
use crate::memo::{
    Attribution, BuilderKeyRequest, ChainKeysVerdict, ColdOutputRequest, ColdReadComparisonRequest,
    ColdReadRequest, ExecutedStep, MemoEngine, NeededPath, OutputMemoRequest, OutputMemoVerdict,
    ReductionRequest, StepMemoKeyRequest, StepReuseRequest, StepReuseVerdict, TopFetchKeyRequest,
    TopReductionRequest,
};
use crate::sandbox::{Sandbox, SandboxRequest};
use crate::trace;
use crate::workspace::{self, Workspace};
use crate::{
    BuildStep, Builder, Cixfile, Fetch, FetchPin, LockFile, ScratchDir, StepMemo, Template,
    TemplatePart, VolatilePath,
};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    cixfile: &Cixfile,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    update: Option<&str>,
    cold: bool,
    allow_secret: bool,
    workspace_directory: &Path,
    codegen: &dyn EvaluationCodegen,
) -> Result<(BTreeMap<String, String>, Vec<ExecutedStep>)> {
    let mut credentials = HostCredentials::load(directory, allow_secret)?;
    let needed = consumed_paths(cixfile);
    let mut binders = BTreeMap::new();
    let mut executed_steps = Vec::new();
    for name in &cixfile.fetch_order {
        let fetch = &cixfile.fetches[name];
        let (view, executed) = execute_top_fetch(
            cixfile,
            name,
            fetch,
            directory,
            lock,
            system,
            &binders,
            needed.get(name).cloned().unwrap_or_default(),
            update.is_some_and(|requested| requested.is_empty() || requested == name),
            cold,
            &mut credentials,
            codegen,
        )?;
        binders.insert(name.clone(), view);
        executed_steps.push(ExecutedStep {
            name: name.clone(),
            kind: "FETCH".into(),
            executed,
        });
    }
    for name in &cixfile.builder_order {
        let builder = &cixfile.builders[name];
        let (view, mut executed) = execute_builder(
            cixfile,
            name,
            builder,
            directory,
            lock,
            system,
            &binders,
            needed.get(name).cloned().unwrap_or_default(),
            update.is_some_and(|requested| requested.is_empty() || requested == name),
            cold,
            workspace_directory,
            &mut credentials,
            codegen,
        )?;
        binders.insert(name.clone(), view);
        executed_steps.append(&mut executed);
    }
    Ok((binders, executed_steps))
}

#[allow(clippy::too_many_arguments)]
fn execute_top_fetch(
    cixfile: &Cixfile,
    name: &str,
    fetch: &Fetch,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    binders: &BTreeMap<String, String>,
    mut needed: BTreeMap<String, NeededPath>,
    force: bool,
    cold: bool,
    credentials: &mut HostCredentials,
    codegen: &dyn EvaluationCodegen,
) -> Result<(String, bool)> {
    let fetch_state = FetchState::new(directory);
    let context_request = FetchContextRequest {
        cixfile,
        name,
        directory,
        lock,
        system,
        snapshots: binders,
    };
    let context = NixEvaluation::fetch_context(codegen, context_request)?;
    // Store paths are complete by store invariant (the ensure_store_path
    // assumption); realization is only needed when an offer is missing.
    if context.offers.iter().any(|path| !Path::new(path).exists()) {
        NixEvaluation::realize_fetch_offers(
            codegen,
            FetchContextRequest {
                cixfile,
                name,
                directory,
                lock,
                system,
                snapshots: binders,
            },
        )?;
    }
    let offered_closure = NixEvaluation::offered_closure(&context.offers)?;
    if context.node.command.requires_shell() {
        Sandbox::shell(&context.imports)?;
    }
    let mut environment = build_environment(context.environment.clone());
    environment.extend(context.node.environment.clone());
    let command = &context.node.command;
    let command_key = command.canonical_text();
    let fetch_id = resolved_statement_id(name, &command_key, cixfile);
    FetchState::install_expected(lock, &fetch_id, fetch.expected.as_deref(), |pin| {
        format!(
            "line {}: FETCH {name:?} EXPECT disagrees with its recorded lock pin\n  | {:?}\n  declared {}\n  lock records {}",
            fetch.line,
            fetch.source,
            fetch.expected.as_deref().expect("EXPECT was supplied"),
            pin.nar_hash
        )
    })?;
    let ignored_evidence = evidence::normalize_paths(&context.node.ignored_evidence)?;
    evidence::report_waivers("FETCH", fetch.line, &fetch.source, &ignored_evidence);
    let universe_identities = context
        .universe_identities
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let trace_key = MemoEngine::trace_key(StepMemoKeyRequest {
        builder: name,
        index: 0,
        kind: "FETCH",
        directive: &fetch.source,
        arguments: &command_key,
        offered_closure: &offered_closure,
        ordered_imports: &context.imports,
        environment: &environment,
        universe_identities: &universe_identities,
    })?;
    let trace_owner = format!("fetch:{fetch_id}");
    if needed.is_empty() {
        needed.insert(".".into(), NeededPath::default());
    }
    let existing_pin = lock.fetches.get(&fetch_id).map(FetchPin::key);
    let existing_key = existing_pin
        .map(|pin| {
            MemoEngine::top_fetch_key(TopFetchKeyRequest {
                command: &command_key,
                offered_closure: &offered_closure,
                environment: &environment,
                pin: &pin,
                universe_identities: &context
                    .universe_identities
                    .values()
                    .cloned()
                    .collect::<Vec<_>>(),
            })
        })
        .transpose()?;
    if cold && !force {
        if let Some(memo) = lock
            .step_memo
            .get(&trace_owner)
            .filter(|memo| memo.key == trace_key)
        {
            let empty = ScratchDir::new("cix-build-cold-")
                .context("creating cold top-level FETCH audit root")?;
            MemoEngine::audit_cold(ColdReadRequest {
                memo,
                workspace: empty.path(),
                line: fetch.line,
                source: &fetch.source,
            })?;
        }
        let pin = lock.fetches.get(&fetch_id).with_context(|| {
            format!("FETCH {name} has no pin to replay; --cold never refetches")
        })?;
        let snapshot = fetch_state.replay_snapshot(&fetch_id, pin)?;
        FetchState::verify(fetch.expected.as_deref(), Some(pin), None)?;
        let paths = workspace::store_consumed_paths_excluding(
            Path::new(&snapshot),
            needed.keys().cloned(),
            &ignored_evidence,
        )?;
        let key = MemoEngine::top_fetch_key(TopFetchKeyRequest {
            command: &command_key,
            offered_closure: &offered_closure,
            environment: &environment,
            pin: &pin.key(),
            universe_identities: &context
                .universe_identities
                .values()
                .cloned()
                .collect::<Vec<_>>(),
        })?;
        lock.memo
            .insert(key.clone(), MemoEngine::entry(paths.clone()));
        let view = workspace::materialize_view(&paths)?;
        eprintln!(
            "FETCH {name} replayed pinned snapshot {} -> {view}",
            short_key(&key)
        );
        return Ok((view, false));
    }
    if !force {
        if let Some(key) = &existing_key {
            if let OutputMemoVerdict::Hit { view } = MemoEngine::output(OutputMemoRequest {
                entry: lock.memo.get(key),
                needed: needed.keys().cloned().collect(),
            })? {
                FetchState::verify(fetch.expected.as_deref(), lock.fetches.get(&fetch_id), None)
                    .with_context(|| {
                        format!(
                            "line {}: top-level FETCH {name:?} pin verification failed\n  | {:?}",
                            fetch.line, fetch.source
                        )
                    })?;
                eprintln!("FETCH {name} memo hit {} -> {view}", short_key(key));
                return Ok((view, false));
            }
        }
    }
    let work = ScratchDir::new("cix-fetch-work-").context("creating top-level FETCH workdir")?;
    let trace_before = fetch_state.snapshot(work.path())?;
    let started = Instant::now();
    let credential = credentials.for_command(&command.credential_text())?;
    let credential_mounts = credential.as_ref().into_iter().collect::<Vec<_>>();
    let observations = Sandbox::execute(SandboxRequest {
        workdir: work.path(),
        command,
        environment: &environment,
        export_prelude: &BTreeMap::new(),
        offered_closure: &offered_closure,
        imports: &context.imports,
        run_network: None,
        credentials: &credential_mounts,
    })
    .with_context(|| {
        format!(
            "line {}: top-level FETCH {name:?} failed\n  | {:?}",
            fetch.line, fetch.source
        )
    })?;
    let mut candidates = trace::unsafe_ignore_candidates(&observations);
    evidence::retain_included_set(&mut candidates, &ignored_evidence);
    evidence::report_candidates("FETCH", fetch.line, &candidates);
    let mut step_volatile = ignored_evidence.clone();
    let volatile = if force && fetch.expected.is_none() {
        let first = fetch_state.snapshot(work.path())?;
        let empty = ScratchDir::new("cix-build-cold-")?;
        workspace::replace_tree_at(empty.path(), work.path())?;
        Sandbox::execute(SandboxRequest {
            workdir: work.path(),
            command,
            environment: &environment,
            export_prelude: &BTreeMap::new(),
            offered_closure: &offered_closure,
            imports: &context.imports,
            run_network: None,
            credentials: &credential_mounts,
        })
        .with_context(|| {
            format!(
                "line {}: top-level FETCH {name:?} probe failed\n  | {:?}",
                fetch.line, fetch.source
            )
        })?;
        let mut observed_volatile = fetch_state.volatile_paths(first.path(), work.path())?;
        evidence::retain_included(&mut observed_volatile, &ignored_evidence);
        FetchState::report_volatility(name, &observed_volatile);
        step_volatile.extend(observed_volatile.keys().cloned());
        workspace::replace_tree_at(first.path(), work.path())?;
        let volatile = FetchState::consumed_volatility(observed_volatile, &needed);
        first.close()?;
        volatile
    } else {
        BTreeMap::new()
    };
    let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let sealed =
        workspace::seal_directory_excluding(work.path(), &ignored_evidence, "cix-fetch-snapshot")?;
    let output_hash = sealed.nar_hash;
    if let Some(expected) = fetch.expected.as_deref() {
        FetchState::verify(Some(expected), None, Some(&output_hash)).with_context(|| {
            format!(
                "line {}: top-level FETCH {name:?} output did not match EXPECT\n  | {:?}",
                fetch.line, fetch.source
            )
        })?;
    } else if force || !lock.fetches.contains_key(&fetch_id) {
        lock.fetches.insert(fetch_id.clone(), FetchPin::automatic());
    }
    let snapshot = sealed.store_path;
    let reads = trace::read_dependencies(trace_before.path(), &observations)?;
    let changes =
        trace::filesystem_changes(trace_before.path(), work.path(), &observations.writes)?;
    let reduced = MemoEngine::reduce_top(TopReductionRequest {
        before: trace_before.path(),
        workspace: work.path(),
        reads,
        changes,
        writes: &observations.writes,
        volatile: &step_volatile,
    })?;
    let changes = reduced.changes;
    let step_output = (!changes.is_empty())
        .then(|| workspace::add_step_output_snapshot(work.path(), &changes, &step_volatile))
        .transpose()?;
    lock.step_memo.insert(
        trace_owner,
        StepMemo {
            key: trace_key,
            reads: reduced.reads,
            output_snapshot: step_output,
            changes,
            output_hashes: reduced.output_hashes,
        },
    );
    trace_before.close()?;
    let actual_paths = FetchState::consumed_path_hashes(work.path(), &needed, &ignored_evidence)?;
    FetchState::report_unconsumed_complement(name, work.path(), &needed);
    let pin = lock.fetches.get(&fetch_id).cloned();
    let refreshed = FetchState::refresh_pin(PinRefreshRequest {
        previous: pin.as_ref(),
        expected: fetch.expected.is_some(),
        force,
        actual_paths,
        snapshot_nar_hash: &output_hash,
        volatile,
        name,
    })?;
    fetch_state.cache_snapshot(&fetch_id, &refreshed, &snapshot)?;
    lock.fetches.insert(fetch_id.clone(), refreshed);
    let pin = lock.fetches[&fetch_id].key();
    let key = MemoEngine::top_fetch_key(TopFetchKeyRequest {
        command: &command_key,
        offered_closure: &offered_closure,
        environment: &environment,
        pin: &pin,
        universe_identities: &context
            .universe_identities
            .values()
            .cloned()
            .collect::<Vec<_>>(),
    })?;
    let paths = workspace::store_consumed_paths_excluding(
        work.path(),
        needed.keys().cloned(),
        &ignored_evidence,
    )?;
    lock.memo
        .insert(key.clone(), MemoEngine::entry(paths.clone()));
    let view = workspace::materialize_view(&paths)?;
    eprintln!(
        "FETCH {name} memo miss {} ({} ms) -> {}",
        short_key(&key),
        wall_ms,
        view
    );
    Ok((view, true))
}

#[allow(clippy::too_many_arguments)]
fn execute_builder(
    cixfile: &Cixfile,
    builder_name: &str,
    builder: &Builder,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    binders: &BTreeMap<String, String>,
    needed: BTreeMap<String, NeededPath>,
    update_fetch_pins: bool,
    cold: bool,
    workspace_directory: &Path,
    credentials: &mut HostCredentials,
    codegen: &dyn EvaluationCodegen,
) -> Result<(String, Vec<ExecutedStep>)> {
    let fetch_state = FetchState::new(directory);
    let node_count = builder
        .steps
        .iter()
        .filter(|step| matches!(step, BuildStep::Fetch { .. } | BuildStep::Run { .. }))
        .count();
    let copy_count = builder
        .steps
        .iter()
        .filter(|step| matches!(step, BuildStep::Copy(_)))
        .count();
    let context = NixEvaluation::builder_context(
        codegen,
        BuilderContextRequest {
            cixfile,
            name: builder_name,
            directory,
            lock,
            system,
            snapshots: binders,
        },
    )?;
    if context.nodes.len() != node_count {
        bail!(
            "internal build context mismatch: resolved {} nodes for {node_count} steps",
            context.nodes.len()
        );
    }
    let mut ignored_by_node = Vec::with_capacity(node_count);
    let mut all_ignored_evidence = BTreeSet::new();
    let mut report_node_index = 0;
    for step in &builder.steps {
        let (kind, line, source) = match step {
            BuildStep::Fetch { line, source, .. } => ("FETCH", *line, source.as_str()),
            BuildStep::Run { line, source, .. } => ("RUN", *line, source.as_str()),
            BuildStep::Env { .. } | BuildStep::Copy(_) => continue,
        };
        let ignored =
            evidence::normalize_paths(&context.nodes[report_node_index].ignored_evidence)?;
        report_node_index += 1;
        evidence::report_waivers(kind, line, source, &ignored);
        all_ignored_evidence.extend(ignored.iter().cloned());
        ignored_by_node.push(ignored);
    }
    if context.copies.len() != copy_count {
        bail!(
            "internal builder context mismatch: resolved {} COPY sources for {copy_count} steps",
            context.copies.len()
        );
    }
    let offered_closure = if context.offers.is_empty() {
        BTreeSet::new()
    } else {
        // Store paths are complete by store invariant (the ensure_store_path
        // assumption); realization is only needed when an offer is missing.
        if context.offers.iter().any(|path| !Path::new(path).exists()) {
            NixEvaluation::realize_builder_offers(
                codegen,
                BuilderContextRequest {
                    cixfile,
                    name: builder_name,
                    directory,
                    lock,
                    system,
                    snapshots: binders,
                },
            )?;
        }
        NixEvaluation::offered_closure(&context.offers)?
    };
    if context
        .nodes
        .iter()
        .any(|node| node.command.requires_shell())
    {
        Sandbox::shell(&context.imports)?;
    }
    let run_network = if builder
        .steps
        .iter()
        .any(|step| matches!(step, BuildStep::Run { .. }))
    {
        Some(Sandbox::run_network()?)
    } else {
        None
    };
    let mut environment = NixEvaluation::development_environment(
        codegen,
        DevEnvironmentRequest {
            cixfile,
            builder_name,
            directory,
            lock,
            system,
            snapshots: binders,
            imports: &context.imports,
            universe_identities: &context.universe_identities,
        },
    )?;
    environment.extend(context.environment.clone());
    environment = build_environment(environment);
    let universe_identities = context
        .universe_identities
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut export_prelude = BTreeMap::new();
    FetchState::install_builder_expectations(lock, cixfile, builder_name, builder, &context.nodes)?;
    let chain_key_started = Instant::now();
    let existing_keys = match MemoEngine::builder_keys(BuilderKeyRequest {
        cixfile,
        builder_name,
        builder,
        nodes: &context.nodes,
        copies: &context.copies,
        ordered_imports: &context.imports,
        universe_identities: &universe_identities,
        offered_closure: &offered_closure,
        environment: &environment,
        lock,
    })? {
        ChainKeysVerdict::Complete(keys) => Some(keys),
        ChainKeysVerdict::UnpinnedFetch => None,
    };
    crate::cix_timing!(
        "CIX timing chain-keys phase=initial wall_ms={}",
        chain_key_started.elapsed().as_millis()
    );
    let existing_key = existing_keys.as_ref().map(|keys| {
        keys.last()
            .cloned()
            .unwrap_or_else(|| hex_hash(format!("BUILDER\0{builder_name}").as_bytes()))
    });
    let newly_consumed_paths = existing_key.as_ref().is_some_and(|key| {
        lock.memo
            .get(key)
            .is_some_and(|entry| needed.keys().any(|path| !entry.paths.contains_key(path)))
    });
    if !cold && !update_fetch_pins {
        if let Some(key) = &existing_key {
            if let OutputMemoVerdict::Hit { view } = MemoEngine::output(OutputMemoRequest {
                entry: lock.memo.get(key),
                needed: needed.keys().cloned().collect(),
            })? {
                eprintln!(
                    "BUILDER {builder_name} memo hit {} -> {view}",
                    short_key(key)
                );
                return Ok((
                    view,
                    MemoEngine::builder_results(builder_name, builder, false),
                ));
            }
        }
    }

    let workspace = if cold {
        Workspace::cold()?
    } else {
        Workspace::persistent(workspace_directory, directory, builder_name)?
    };
    if !cold {
        eprintln!(
            "BUILDER {builder_name} workspace {}",
            workspace.path().display()
        );
    }
    let mut memo_engine = MemoEngine::new(&workspace);
    let rerun_from = memo_engine.rerun_from(existing_keys.as_deref(), cold, update_fetch_pins);
    let workdir = workspace.path();

    let mut node_index = 0;
    let mut copy_index = 0;
    let mut step_results = Vec::with_capacity(builder.steps.len());
    let mut fetch_snapshots = BTreeMap::<
        String,
        (
            bool,
            Option<String>,
            String,
            BTreeMap<String, VolatilePath>,
            BTreeSet<String>,
        ),
    >::new();
    for (index, step) in builder.steps.iter().enumerate() {
        match step {
            BuildStep::Env {
                name,
                value,
                line,
                source,
            } => {
                let value = value
                    .literal_value()
                    .context("builder ENV metadata was not resolved")?;
                environment.insert(name.clone(), value.clone());
                export_prelude.insert(name.clone(), value);
                eprintln!(
                    "BUILDER {builder_name} step {} ENV {name} declared (line {line}: {source})",
                    index + 1
                );
                step_results.push(MemoEngine::step_result(builder_name, index, "ENV", true));
            }
            BuildStep::Copy(copy) => {
                let resolved_source = &context.copies[copy_index];
                copy_index += 1;
                let staging_started = Instant::now();
                workspace
                    .stage_input(Path::new(resolved_source), &copy.dst, index)
                    .with_context(|| {
                        format!("line {}: COPY failed\n  | {:?}", copy.line, copy.source)
                    })?;
                crate::cix_timing!(
                    "CIX timing COPY step={} wall_ms={}",
                    index + 1,
                    staging_started.elapsed().as_millis()
                );
                eprintln!(
                    "BUILDER {builder_name} step {} COPY {} -> {}",
                    index + 1,
                    resolved_source,
                    copy.dst
                );
                step_results.push(MemoEngine::step_result(builder_name, index, "COPY", true));
            }
            BuildStep::Fetch { line, source, .. } | BuildStep::Run { line, source, .. } => {
                let node = &context.nodes[node_index];
                let ignored_evidence = &ignored_by_node[node_index];
                node_index += 1;
                let command = &node.command;
                let command_key = command.canonical_text();
                let mut node_environment = environment.clone();
                node_environment.extend(node.environment.clone());
                if index < rerun_from {
                    eprintln!(
                        "BUILDER {builder_name} step {} reused from persistent workspace",
                        index + 1
                    );
                    let kind = if matches!(step, BuildStep::Fetch { .. }) {
                        "FETCH"
                    } else {
                        "RUN"
                    };
                    step_results.push(MemoEngine::step_result(builder_name, index, kind, false));
                    continue;
                }
                let is_fetch = matches!(step, BuildStep::Fetch { .. });
                let kind = if is_fetch { "FETCH" } else { "RUN" };
                let memo_key = MemoEngine::trace_key(StepMemoKeyRequest {
                    builder: builder_name,
                    index,
                    kind,
                    directive: source,
                    arguments: &command_key,
                    offered_closure: &offered_closure,
                    ordered_imports: &context.imports,
                    environment: &node_environment,
                    universe_identities: &universe_identities,
                })?;
                let memo_owner = resolved_statement_id(
                    &format!("builder:{builder_name}:{index}"),
                    &command_key,
                    cixfile,
                );
                let superseded_memo = lock.step_memo.get(&memo_owner).cloned();
                let recorded_memo = superseded_memo
                    .as_ref()
                    .filter(|memo| memo.key == memo_key)
                    .cloned();
                let fetch_id = is_fetch.then(|| {
                    resolved_statement_id(
                        &builder_fetch_id(builder_name, index, &command_key),
                        &command_key,
                        cixfile,
                    )
                });
                if let Some(id) = &fetch_id {
                    if cold {
                        if let Some(memo) = &recorded_memo {
                            memo_engine.audit_cold_reads(memo, *line, source)?;
                        }
                        let pin = lock.fetches.get(id).with_context(|| {
                            format!(
                                "BUILDER {builder_name} FETCH has no pin to replay; --cold never refetches"
                            )
                        })?;
                        if let Some(memo) = &recorded_memo {
                            memo_engine.replay_cold(memo)?;
                        } else {
                            let snapshot = fetch_state.replay_snapshot(id, pin)?;
                            workspace.restore_snapshot(Path::new(&snapshot))?;
                        }
                        eprintln!(
                            "BUILDER {builder_name} step {} FETCH replayed pinned snapshot",
                            index + 1
                        );
                        step_results.push(MemoEngine::step_result(
                            builder_name,
                            index,
                            kind,
                            false,
                        ));
                        continue;
                    }
                }
                let known_reads = match memo_engine.try_reuse(StepReuseRequest {
                    owner: &memo_owner,
                    key: &memo_key,
                    memo: recorded_memo.as_ref(),
                    is_fetch,
                    validate: !cold && !update_fetch_pins,
                    allow_reuse: !newly_consumed_paths,
                })? {
                    StepReuseVerdict::Reused => {
                        eprintln!(
                            "BUILDER {builder_name} step {} {kind} memo hit {}",
                            index + 1,
                            short_key(&memo_key)
                        );
                        step_results.push(MemoEngine::step_result(
                            builder_name,
                            index,
                            kind,
                            false,
                        ));
                        continue;
                    }
                    StepReuseVerdict::Execute { known_reads } => known_reads,
                };
                if is_fetch && !cold {
                    if let Some(memo) = &superseded_memo {
                        crate::cix_timing!(
                            "CIX timing fetch-revert owner={} key={}",
                            memo_owner,
                            short_key(&memo.key)
                        );
                        memo_engine.revert(memo)?;
                    }
                }
                let snapshot_started = Instant::now();
                let trace_before = fetch_state.snapshot(workdir)?;
                crate::cix_timing!(
                    "CIX timing workspace-snapshot phase=before-command wall_ms={}",
                    snapshot_started.elapsed().as_millis()
                );
                let probe_before = (is_fetch
                    && update_fetch_pins
                    && matches!(step, BuildStep::Fetch { expected: None, .. }))
                .then(|| fetch_state.snapshot(workdir))
                .transpose()?;
                let started = Instant::now();
                let credential = if is_fetch {
                    credentials.for_command(&command.credential_text())?
                } else {
                    None
                };
                let credential_mounts = credential.as_ref().into_iter().collect::<Vec<_>>();
                let observations = Sandbox::execute(SandboxRequest {
                    workdir,
                    command,
                    environment: &node_environment,
                    export_prelude: &export_prelude,
                    offered_closure: &offered_closure,
                    imports: &context.imports,
                    run_network: if is_fetch { None } else { run_network },
                    credentials: &credential_mounts,
                })
                .with_context(|| format!("line {line}: {kind} failed\n  | {source:?}"))?;
                let mut candidates = trace::unsafe_ignore_candidates(&observations);
                evidence::retain_included_set(&mut candidates, ignored_evidence);
                evidence::report_candidates(kind, *line, &candidates);
                let read_set_started = Instant::now();
                let empty_reads = BTreeMap::new();
                let (reads, recording_metrics) = trace::read_dependencies_with_known(
                    trace_before.path(),
                    &observations,
                    known_reads.as_ref().unwrap_or(&empty_reads),
                )
                .with_context(|| {
                    format!("line {line}: recording {kind} read set\n  | {source:?}")
                })?;
                crate::cix_timing!(
                    "CIX timing trace-read-set reused={} hashed_files={} hashed_bytes={} hashed_directories={} wall_ms={}",
                    recording_metrics.reused,
                    recording_metrics.hashed_files,
                    recording_metrics.hashed_bytes,
                    recording_metrics.hashed_directories,
                    read_set_started.elapsed().as_millis()
                );
                let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let mut step_volatile = ignored_evidence.clone();
                if is_fetch {
                    let id = fetch_id.expect("FETCH has an id");
                    let volatile = if let Some(before) = probe_before {
                        let first = fetch_state.snapshot(workdir)?;
                        workspace.replace_tree(before.path())?;
                        let _ = Sandbox::execute(SandboxRequest {
                            workdir,
                            command,
                            environment: &node_environment,
                            export_prelude: &export_prelude,
                            offered_closure: &offered_closure,
                            imports: &context.imports,
                            run_network: None,
                            credentials: &credential_mounts,
                        })
                        .with_context(|| {
                            format!("line {line}: FETCH update probe failed\n  | {source:?}")
                        })?;
                        let mut observed_volatile =
                            fetch_state.volatile_paths(first.path(), workdir)?;
                        evidence::retain_included(&mut observed_volatile, ignored_evidence);
                        FetchState::report_volatility(&id, &observed_volatile);
                        step_volatile.extend(observed_volatile.keys().cloned());
                        workspace.replace_tree(first.path())?;
                        before.close()?;
                        let volatile = FetchState::consumed_volatility(observed_volatile, &needed);
                        first.close()?;
                        volatile
                    } else {
                        BTreeMap::new()
                    };
                    let sealed = workspace::seal_directory_excluding(
                        workdir,
                        ignored_evidence,
                        "cix-fetch-snapshot",
                    )?;
                    let actual = sealed.nar_hash;
                    let expected = match step {
                        BuildStep::Fetch { expected, .. } => expected.as_deref(),
                        _ => None,
                    };
                    if let Some(expected) = expected {
                        FetchState::verify(Some(expected), None, Some(&actual)).with_context(
                            || {
                                format!(
                                "line {line}: FETCH output did not match EXPECT\n  | {source:?}"
                            )
                            },
                        )?;
                    } else if !lock.fetches.contains_key(&id) {
                        lock.fetches.insert(id.clone(), FetchPin::automatic());
                    }
                    fetch_snapshots.insert(
                        id,
                        (
                            expected.is_some(),
                            Some(sealed.store_path),
                            actual,
                            volatile,
                            ignored_evidence.clone(),
                        ),
                    );
                }
                let changes_started = Instant::now();
                let changes =
                    trace::filesystem_changes(trace_before.path(), workdir, &observations.writes)?;
                crate::cix_timing!(
                    "CIX timing workspace-delta wall_ms={}",
                    changes_started.elapsed().as_millis()
                );
                let reduced = memo_engine.reduce(ReductionRequest {
                    previous: recorded_memo.as_ref(),
                    before: trace_before.path(),
                    is_fetch,
                    reads,
                    changes,
                    writes: &observations.writes,
                    volatile: &step_volatile,
                })?;
                let reads = reduced.reads;
                let changes = reduced.changes;
                if !is_fetch {
                    memo_engine.invalidate_fetch_outputs(lock, &changes);
                }
                let output_hashes = reduced.output_hashes;
                let output_fingerprints = reduced.output_fingerprints;
                if cold {
                    if let Some(recorded) = &recorded_memo {
                        MemoEngine::compare_cold_reads(ColdReadComparisonRequest {
                            memo: recorded,
                            cold: &reads,
                            line: *line,
                            source,
                        })?;
                    }
                    trace_before.close()?;
                } else {
                    // Parallel independent I/O: removing the workspace-sized
                    // pre-command probe snapshot and store-adding the output
                    // delta touch disjoint trees and share no state; both
                    // results are joined synchronously before the memo lands.
                    let (output_snapshot, closed) = std::thread::scope(|scope| {
                        let closer = scope.spawn(|| {
                            let cleanup_started = Instant::now();
                            let result = trace_before.close();
                            crate::cix_timing!(
                                "CIX timing probe-cleanup wall_ms={}",
                                cleanup_started.elapsed().as_millis()
                            );
                            result
                        });
                        // FETCH memos stay constructive (replay is what makes
                        // pins and --cold work without the network); RUN memos
                        // are verifying-only: re-executing a RUN in its warm
                        // workspace is exactly the cheap path this feature
                        // optimizes, while snapshotting its result would
                        // store-add the entire warm output tree (measured:
                        // 512 MiB of target/ per one-line gitsitter edit) on
                        // every executed step. Read capture is unaffected.
                        // (Mathijs, 2026-08-02, coarse-grain prompt in chat.)
                        let output_snapshot = if changes.is_empty() || !is_fetch {
                            Ok(None)
                        } else {
                            let output_snapshot_started = Instant::now();
                            let snapshot =
                                workspace.add_step_output_snapshot(&changes, &step_volatile);
                            crate::cix_timing!(
                                "CIX timing output-snapshot wall_ms={}",
                                output_snapshot_started.elapsed().as_millis()
                            );
                            snapshot.map(Some)
                        };
                        (
                            output_snapshot,
                            closer
                                .join()
                                .expect("probe snapshot cleanup thread panicked"),
                        )
                    });
                    let output_snapshot = output_snapshot?;
                    closed?;
                    memo_engine.record_materialized(
                        memo_owner.clone(),
                        memo_key.clone(),
                        is_fetch.then_some(output_fingerprints),
                    );
                    lock.step_memo.insert(
                        memo_owner,
                        StepMemo {
                            key: memo_key,
                            reads,
                            output_snapshot,
                            changes,
                            output_hashes,
                        },
                    );
                }
                eprintln!(
                    "BUILDER {builder_name} step {} {kind} executed ({} ms)",
                    index + 1,
                    wall_ms
                );
                step_results.push(MemoEngine::step_result(builder_name, index, kind, true));
            }
        }
    }
    if !fetch_snapshots.is_empty() {
        FetchState::report_unconsumed_complement(builder_name, workdir, &needed);
        for (id, (expected, snapshot, snapshot_nar_hash, volatile, ignored_evidence)) in
            fetch_snapshots
        {
            let actual_paths =
                FetchState::consumed_path_hashes(workdir, &needed, &ignored_evidence)?;
            let refreshed = FetchState::refresh_pin(PinRefreshRequest {
                previous: lock.fetches.get(&id),
                expected,
                force: update_fetch_pins,
                actual_paths: actual_paths.clone(),
                snapshot_nar_hash: &snapshot_nar_hash,
                volatile,
                name: &id,
            })?;
            if let Some(snapshot) = snapshot {
                fetch_state.cache_snapshot(&id, &refreshed, &snapshot)?;
            }
            lock.fetches.insert(id, refreshed);
        }
    }
    let chain_key_started = Instant::now();
    let step_keys = match MemoEngine::builder_keys(BuilderKeyRequest {
        cixfile,
        builder_name,
        builder,
        nodes: &context.nodes,
        copies: &context.copies,
        ordered_imports: &context.imports,
        universe_identities: &universe_identities,
        offered_closure: &offered_closure,
        environment: &environment,
        lock,
    })? {
        ChainKeysVerdict::Complete(keys) => keys,
        ChainKeysVerdict::UnpinnedFetch => {
            bail!("builder chain still has an unpinned FETCH after execution")
        }
    };
    crate::cix_timing!(
        "CIX timing chain-keys phase=final wall_ms={}",
        chain_key_started.elapsed().as_millis()
    );
    let key = step_keys
        .last()
        .cloned()
        .unwrap_or_else(|| hex_hash(format!("BUILDER\0{builder_name}").as_bytes()));
    let paths = workspace::store_consumed_paths_excluding(
        workdir,
        needed.keys().cloned(),
        &all_ignored_evidence,
    )?;
    if cold {
        MemoEngine::compare_cold_outputs(ColdOutputRequest {
            warm: lock.memo.get(&key),
            cold: &paths,
            needed: &needed,
        })?;
    }
    lock.memo
        .insert(key.clone(), MemoEngine::entry(paths.clone()));
    if !cold {
        memo_engine.finish(step_keys.clone(), lock)?;
    }
    let view = workspace::materialize_view(&paths)?;
    eprintln!(
        "BUILDER {builder_name} memo miss {} -> {view}",
        short_key(&key)
    );
    Ok((view, step_results))
}

fn build_environment(mut environment: BTreeMap<String, String>) -> BTreeMap<String, String> {
    environment.insert("HOME".into(), "/work".into());
    environment.insert("LC_ALL".into(), "C".into());
    environment.insert("PATH".into(), "/bin".into());
    environment.insert("SOURCE_DATE_EPOCH".into(), "1".into());
    environment.insert(
        "SSL_CERT_FILE".into(),
        "/etc/ssl/certs/ca-bundle.crt".into(),
    );
    environment.insert("TMPDIR".into(), "/tmp".into());
    environment.insert("TZ".into(), "UTC".into());
    environment
}

fn consumed_paths(cixfile: &Cixfile) -> BTreeMap<String, BTreeMap<String, NeededPath>> {
    let mut needed = BTreeMap::<String, BTreeMap<String, NeededPath>>::new();
    let mut add = |binder: &str, path: &str, attribution: Option<Attribution>| {
        if !cixfile.builders.contains_key(binder) && !cixfile.fetches.contains_key(binder) {
            return;
        }
        let record = needed
            .entry(binder.to_owned())
            .or_default()
            .entry(path.to_owned())
            .or_default();
        if let Some(attribution) = attribution {
            record.attributions.push(attribution);
        }
    };
    for artifact in cixfile.artifacts.values() {
        for import in &artifact.imports {
            for binder in template_binders(import) {
                add(binder, ".", None);
            }
        }
        for copy in &artifact.copies {
            if let Some((binder, path)) = binder_path(&copy.src) {
                add(
                    binder,
                    path,
                    Some(Attribution {
                        binder: binder.to_owned(),
                        path: path.to_owned(),
                        line: copy.line,
                    }),
                );
            }
        }
    }
    for builder in cixfile.builders.values() {
        for template in &builder.imports {
            for binder in template_binders(template) {
                add(binder, ".", None);
            }
        }
        for step in &builder.steps {
            match step {
                BuildStep::Env { .. } => {}
                BuildStep::Copy(copy) => {
                    if let Some((binder, path)) = binder_path(&copy.src) {
                        add(
                            binder,
                            path,
                            Some(Attribution {
                                binder: binder.to_owned(),
                                path: path.to_owned(),
                                line: copy.line,
                            }),
                        );
                    }
                }
                BuildStep::Fetch {
                    command,
                    environment,
                    ..
                }
                | BuildStep::Run {
                    command,
                    environment,
                    ..
                } => {
                    for template in command.templates() {
                        for binder in template_binders(template) {
                            add(binder, ".", None);
                        }
                    }
                    for template in environment.values() {
                        for binder in template_binders(template) {
                            add(binder, ".", None);
                        }
                    }
                }
            }
        }
    }
    for fetch in cixfile.fetches.values() {
        for template in fetch
            .command
            .templates()
            .into_iter()
            .chain(fetch.environment.values())
        {
            for binder in template_binders(template) {
                add(binder, ".", None);
            }
        }
    }
    needed
}

fn binder_path(template: &Template) -> Option<(&str, &str)> {
    match template.parts.as_slice() {
        [TemplatePart::Binder { name, .. }] => Some((name, ".")),
        [TemplatePart::Binder { name, .. }, TemplatePart::Literal(path)] => {
            let path = path.trim_start_matches('/');
            Some((name, if path.is_empty() { "." } else { path }))
        }
        _ => None,
    }
}

fn template_binders(template: &Template) -> impl Iterator<Item = &str> {
    template.parts.iter().filter_map(|part| match part {
        TemplatePart::Binder { name, .. } => Some(name.as_str()),
        _ => None,
    })
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn short_key(key: &str) -> &str {
    &key[..12.min(key.len())]
}

#[cfg(test)]
#[path = "build_chain_tests.rs"]
mod tests;
