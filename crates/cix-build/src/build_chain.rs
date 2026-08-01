use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::codegen::{
    generate_builder_context_nix, generate_builder_offer_nix, generate_fetch_context_nix,
    generate_fetch_offer_nix,
};
use crate::seccomp;
use crate::{
    BuildStep, Builder, Cixfile, ConsumedPath, Fetch, FetchPin, LockFile, MemoEntry, Template,
    TemplatePart, VolatilePath,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildContext {
    offers: Vec<String>,
    imports: Vec<String>,
    commands: Vec<String>,
    copies: Vec<String>,
    environment: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct StepKeyRequest<'a> {
    kind: &'a str,
    arguments: &'a str,
    offered_closure: &'a BTreeSet<String>,
    ordered_imports: &'a [String],
    predecessor: &'a str,
    declared_sources: &'a [String],
    environment: &'a BTreeMap<String, String>,
    fetch_pin: Option<String>,
}

// Bump whenever the fixed bubblewrap filesystem skeleton changes: memoized
// commands must not be reused across a different execution environment.
const SANDBOX_SKELETON: &str = "v1:/usr/bin/env->/bin/env";
// Bump this when codegen-relevant Cixfile semantics change without a package
// version bump.  It keeps memo keys isolated across concurrently-built checkouts.
const CODEGEN_FINGERPRINT: &str = concat!(env!("CARGO_PKG_VERSION"), ":d69-v1");

#[derive(Clone, Debug, Default)]
struct NeededPath {
    attributions: Vec<Attribution>,
}

#[derive(Clone, Debug)]
struct Attribution {
    binder: String,
    path: String,
    line: usize,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceState {
    step_keys: Vec<String>,
}

struct FetchProbe {
    temporary: Option<tempfile::TempDir>,
}

impl FetchProbe {
    fn path(&self) -> &Path {
        self.temporary
            .as_ref()
            .expect("FETCH probe snapshot is open")
            .path()
    }

    fn close(mut self) -> Result<()> {
        cleanup_fetch_probe(self.temporary.take().expect("FETCH probe snapshot is open"))
    }
}

impl Drop for FetchProbe {
    fn drop(&mut self) {
        let Some(temporary) = self.temporary.take() else {
            return;
        };
        if let Err(error) = cleanup_fetch_probe(temporary) {
            eprintln!("warning: failed to clean FETCH probe snapshot: {error:#}");
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunNetwork {
    Namespace,
    SocketFilter,
}

pub fn execute(
    cixfile: &Cixfile,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    update: Option<&str>,
    cold: bool,
) -> Result<BTreeMap<String, String>> {
    let needed = consumed_paths(cixfile);
    let mut binders = BTreeMap::new();
    for name in &cixfile.fetch_order {
        let fetch = &cixfile.fetches[name];
        let view = execute_top_fetch(
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
        )?;
        binders.insert(name.clone(), view);
    }
    for name in &cixfile.builder_order {
        let builder = &cixfile.builders[name];
        let view = execute_builder(
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
        )?;
        binders.insert(name.clone(), view);
    }
    Ok(binders)
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
) -> Result<String> {
    if let Some(expected) = &fetch.expected {
        if lock.fetches.get(name).map(|pin| &pin.nar_hash) != Some(expected) {
            lock.fetches
                .insert(name.to_owned(), FetchPin::expected(expected.clone()));
        }
    }
    let context = resolve_fetch_context(cixfile, name, directory, lock, system, binders)?;
    if context.commands.len() != 1 {
        bail!(
            "internal top-level FETCH context mismatch: resolved {} commands",
            context.commands.len()
        );
    }
    realize_fetch_offers(cixfile, name, directory, lock, system, binders)?;
    let offered_closure = query_closure(&context.offers)?;
    let shell = find_shell(&context.imports)?;
    let environment = build_environment(context.environment.clone());
    let command = &context.commands[0];
    if needed.is_empty() {
        needed.insert(".".into(), NeededPath::default());
    }
    let existing_pin = lock.fetches.get(name).map(FetchPin::key);
    let existing_key = existing_pin
        .map(|pin| top_fetch_chain_key(command, &offered_closure, &environment, &pin))
        .transpose()?;
    if cold && !force {
        let pin = lock.fetches.get(name).with_context(|| {
            format!("FETCH {name} has no pin to replay; --cold never refetches")
        })?;
        let snapshot = replay_fetch_snapshot(directory, name, pin)?;
        verify_fetch_hash(fetch.expected.as_deref(), Some(pin), None)?;
        let paths = store_consumed_paths(Path::new(&snapshot), &needed)?;
        let key = top_fetch_chain_key(command, &offered_closure, &environment, &pin.key())?;
        lock.memo.insert(key.clone(), memo_entry(paths.clone()));
        let view = materialize_view(&paths)?;
        eprintln!(
            "FETCH {name} replayed pinned snapshot {} -> {view}",
            short_key(&key)
        );
        return Ok(view);
    }
    if !force {
        if let Some(key) = &existing_key {
            if memo_has_paths(lock.memo.get(key), &needed)? {
                let entry = &lock.memo[key];
                verify_fetch_hash(fetch.expected.as_deref(), lock.fetches.get(name), None)
                    .with_context(|| {
                        format!(
                            "line {}: top-level FETCH {name:?} pin verification failed\n  | {:?}",
                            fetch.line, fetch.source
                        )
                    })?;
                let view = materialize_view(&entry.paths)?;
                eprintln!("FETCH {name} memo hit {} -> {view}", short_key(key));
                return Ok(view);
            }
        }
    }
    let work = tempfile::Builder::new()
        .prefix("cix-fetch-work-")
        .tempdir()
        .context("creating top-level FETCH workdir")?;
    let started = Instant::now();
    run_sandbox(
        work.path(),
        &shell,
        command,
        &environment,
        &BTreeMap::new(),
        &offered_closure,
        &context.imports,
        None,
    )
    .with_context(|| {
        format!(
            "line {}: top-level FETCH {name:?} failed\n  | {:?}",
            fetch.line, fetch.source
        )
    })?;
    let volatile = if force && fetch.expected.is_none() {
        let first = copied_snapshot(work.path())?;
        let empty = tempfile::tempdir()?;
        replace_workspace_tree(empty.path(), work.path())?;
        run_sandbox(
            work.path(),
            &shell,
            command,
            &environment,
            &BTreeMap::new(),
            &offered_closure,
            &context.imports,
            None,
        )
        .with_context(|| {
            format!(
                "line {}: top-level FETCH {name:?} probe failed\n  | {:?}",
                fetch.line, fetch.source
            )
        })?;
        let observed_volatile = volatile_paths(first.path(), work.path())?;
        report_volatile(name, &observed_volatile);
        replace_workspace_tree(first.path(), work.path())?;
        let volatile = consumed_volatile_paths(observed_volatile, &needed);
        first.close()?;
        volatile
    } else {
        BTreeMap::new()
    };
    let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let output_hash = nar_hash(work.path())?;
    if let Some(expected) = fetch.expected.as_deref() {
        verify_fetch_hash(Some(expected), None, Some(&output_hash)).with_context(|| {
            format!(
                "line {}: top-level FETCH {name:?} output did not match EXPECT\n  | {:?}",
                fetch.line, fetch.source
            )
        })?;
    } else if force || !lock.fetches.contains_key(name) {
        lock.fetches.insert(name.to_owned(), FetchPin::automatic());
    }
    let snapshot = add_store_object(work.path(), "cix-fetch-snapshot")?;
    let actual_paths = fetch_path_hashes(work.path(), &needed)?;
    let pin = lock.fetches.get(name).cloned();
    let refreshed = refresh_fetch_pin(
        pin.as_ref(),
        fetch.expected.is_some(),
        force,
        actual_paths,
        volatile,
        name,
    )?;
    cache_fetch_snapshot(directory, name, &refreshed, &snapshot)?;
    lock.fetches.insert(name.to_owned(), refreshed);
    let pin = lock.fetches[name].key();
    let key = top_fetch_chain_key(command, &offered_closure, &environment, &pin)?;
    let paths = store_consumed_paths(work.path(), &needed)?;
    lock.memo.insert(key.clone(), memo_entry(paths.clone()));
    let view = materialize_view(&paths)?;
    eprintln!(
        "FETCH {name} memo miss {} ({} ms) -> {}",
        short_key(&key),
        wall_ms,
        view
    );
    Ok(view)
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
) -> Result<String> {
    let command_count = builder
        .steps
        .iter()
        .filter(|step| matches!(step, BuildStep::Fetch { .. } | BuildStep::Run { .. }))
        .count();
    let copy_count = builder
        .steps
        .iter()
        .filter(|step| matches!(step, BuildStep::Copy(_)))
        .count();
    let context = resolve_builder_context(cixfile, builder_name, directory, lock, system, binders)?;
    if context.commands.len() != command_count {
        bail!(
            "internal build context mismatch: resolved {} commands for {command_count} steps",
            context.commands.len()
        );
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
        realize_builder_offers(cixfile, builder_name, directory, lock, system, binders)?;
        query_closure(&context.offers)?
    };
    let shell = if command_count == 0 {
        None
    } else {
        Some(find_shell(&context.imports)?)
    };
    let run_network = if builder
        .steps
        .iter()
        .any(|step| matches!(step, BuildStep::Run { .. }))
    {
        Some(probe_run_network(
            shell.as_deref().expect("RUN steps have a shell"),
        )?)
    } else {
        None
    };
    let mut environment = build_environment(context.environment.clone());
    let mut export_prelude = BTreeMap::new();
    install_declared_expectations(builder_name, builder, &context.commands, lock);
    let existing_keys = builder_chain_keys(
        builder_name,
        builder,
        &context,
        &offered_closure,
        &environment,
        lock,
    )?;
    let existing_key = existing_keys.as_ref().map(|keys| {
        keys.last()
            .cloned()
            .unwrap_or_else(|| hex_hash(format!("BUILDER\0{builder_name}").as_bytes()))
    });
    if !cold && !update_fetch_pins {
        if let Some(key) = &existing_key {
            if memo_has_paths(lock.memo.get(key), &needed)? {
                let view = materialize_view(&lock.memo[key].paths)?;
                eprintln!(
                    "BUILDER {builder_name} memo hit {} -> {view}",
                    short_key(key)
                );
                return Ok(view);
            }
        }
    }

    let persistent = (!cold)
        .then(|| workspace_paths(directory, builder_name))
        .transpose()?;
    let prior_keys = persistent
        .as_ref()
        .and_then(|paths| load_workspace_state(&paths.2))
        .map(|state| state.step_keys)
        .unwrap_or_default();
    let first_changed = existing_keys.as_ref().map_or(0, |keys| {
        keys.iter()
            .zip(&prior_keys)
            .take_while(|(current, prior)| current == prior)
            .count()
    });
    let warm_rerun_from = existing_keys
        .as_ref()
        .filter(|keys| keys.as_slice() != prior_keys.as_slice())
        .map_or(0, |_| first_changed);
    let rerun_from = if cold || update_fetch_pins {
        0
    } else {
        warm_rerun_from
    };
    let temporary;
    let (workdir, staging) = if cold {
        temporary = tempfile::Builder::new()
            .prefix("cix-build-cold-")
            .tempdir()
            .context("creating cold builder workspace")?;
        let staging = temporary.path().join("staged");
        let work = temporary.path().join("work");
        fs::create_dir_all(&staging)?;
        fs::create_dir_all(&work)?;
        (work, staging)
    } else {
        let persistent = persistent.as_ref().expect("warm execution has a workspace");
        eprintln!(
            "BUILDER {builder_name} workspace {}",
            persistent.0.display()
        );
        (persistent.0.clone(), persistent.1.clone())
    };

    let mut command_index = 0;
    let mut copy_index = 0;
    let mut fetch_snapshots =
        BTreeMap::<String, (bool, Option<String>, BTreeMap<String, VolatilePath>)>::new();
    for (index, step) in builder.steps.iter().enumerate() {
        match step {
            BuildStep::Env {
                name,
                value,
                line,
                source,
            } => {
                environment.insert(name.clone(), value.clone());
                export_prelude.insert(name.clone(), value.clone());
                eprintln!(
                    "BUILDER {builder_name} step {} ENV {name} declared (line {line}: {source})",
                    index + 1
                );
            }
            BuildStep::Copy(copy) => {
                let resolved_source = &context.copies[copy_index];
                copy_index += 1;
                stage_input(
                    Path::new(resolved_source),
                    &copy.dst,
                    &workdir,
                    &staging.join(format!("step-{index}")),
                )
                .with_context(|| {
                    format!("line {}: COPY failed\n  | {:?}", copy.line, copy.source)
                })?;
                eprintln!(
                    "BUILDER {builder_name} step {} COPY {} -> {}",
                    index + 1,
                    resolved_source,
                    copy.dst
                );
            }
            BuildStep::Fetch { line, source, .. } | BuildStep::Run { line, source, .. } => {
                let command = &context.commands[command_index];
                command_index += 1;
                if index < rerun_from {
                    eprintln!(
                        "BUILDER {builder_name} step {} reused from persistent workspace",
                        index + 1
                    );
                    continue;
                }
                let is_fetch = matches!(step, BuildStep::Fetch { .. });
                let kind = if is_fetch { "FETCH" } else { "RUN" };
                let fetch_id = is_fetch
                    .then(|| format!("builder:{builder_name}:{}", fetch_id(index, command)));
                if let Some(id) = &fetch_id {
                    if cold {
                        let pin = lock.fetches.get(id).with_context(|| {
                            format!(
                                "BUILDER {builder_name} FETCH has no pin to replay; --cold never refetches"
                            )
                        })?;
                        let snapshot = replay_fetch_snapshot(directory, id, pin)?;
                        restore_snapshot(Path::new(&snapshot), &workdir)?;
                        fetch_snapshots.insert(
                            id.clone(),
                            (
                                matches!(
                                    step,
                                    BuildStep::Fetch {
                                        expected: Some(_),
                                        ..
                                    }
                                ),
                                None,
                                pin.volatile.clone(),
                            ),
                        );
                        eprintln!(
                            "BUILDER {builder_name} step {} FETCH replayed pinned snapshot",
                            index + 1
                        );
                        continue;
                    }
                }
                let probe_before = (is_fetch
                    && update_fetch_pins
                    && matches!(step, BuildStep::Fetch { expected: None, .. }))
                .then(|| copied_snapshot(&workdir))
                .transpose()?;
                let started = Instant::now();
                run_sandbox(
                    &workdir,
                    shell.as_deref().expect("command steps have a shell"),
                    command,
                    &environment,
                    &export_prelude,
                    &offered_closure,
                    &context.imports,
                    if is_fetch { None } else { run_network },
                )
                .with_context(|| format!("line {line}: {kind} failed\n  | {source:?}"))?;
                let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if is_fetch {
                    let id = fetch_id.expect("FETCH has an id");
                    let volatile = if let Some(before) = probe_before {
                        let first = copied_snapshot(&workdir)?;
                        replace_workspace_tree(before.path(), &workdir)?;
                        run_sandbox(
                            &workdir,
                            shell.as_deref().expect("command steps have a shell"),
                            command,
                            &environment,
                            &export_prelude,
                            &offered_closure,
                            &context.imports,
                            None,
                        )
                        .with_context(|| {
                            format!("line {line}: FETCH update probe failed\n  | {source:?}")
                        })?;
                        let observed_volatile = volatile_paths(first.path(), &workdir)?;
                        report_volatile(&id, &observed_volatile);
                        replace_workspace_tree(first.path(), &workdir)?;
                        before.close()?;
                        let volatile = consumed_volatile_paths(observed_volatile, &needed);
                        first.close()?;
                        volatile
                    } else {
                        BTreeMap::new()
                    };
                    let actual = nar_hash(&workdir)?;
                    let expected = match step {
                        BuildStep::Fetch { expected, .. } => expected.as_deref(),
                        _ => None,
                    };
                    if let Some(expected) = expected {
                        verify_fetch_hash(Some(expected), None, Some(&actual)).with_context(
                            || {
                                format!(
                                "line {line}: FETCH output did not match EXPECT\n  | {source:?}"
                            )
                            },
                        )?;
                    } else if !lock.fetches.contains_key(&id) {
                        lock.fetches.insert(id.clone(), FetchPin::automatic());
                    }
                    let snapshot = add_store_object(&workdir, "cix-fetch-snapshot")?;
                    fetch_snapshots.insert(id, (expected.is_some(), Some(snapshot), volatile));
                }
                eprintln!(
                    "BUILDER {builder_name} step {} {kind} executed ({} ms)",
                    index + 1,
                    wall_ms
                );
            }
        }
    }
    if !fetch_snapshots.is_empty() {
        let actual_paths = fetch_path_hashes(&workdir, &needed)?;
        for (id, (expected, snapshot, volatile)) in fetch_snapshots {
            let refreshed = refresh_fetch_pin(
                lock.fetches.get(&id),
                expected,
                update_fetch_pins,
                actual_paths.clone(),
                volatile,
                &id,
            )?;
            if let Some(snapshot) = snapshot {
                cache_fetch_snapshot(directory, &id, &refreshed, &snapshot)?;
            }
            lock.fetches.insert(id, refreshed);
        }
    }
    let step_keys = builder_chain_keys(
        builder_name,
        builder,
        &context,
        &offered_closure,
        &environment,
        lock,
    )?
    .context("builder chain still has an unpinned FETCH after execution")?;
    let key = step_keys
        .last()
        .cloned()
        .unwrap_or_else(|| hex_hash(format!("BUILDER\0{builder_name}").as_bytes()));
    let paths = store_consumed_paths(&workdir, &needed)?;
    if cold {
        compare_cold_paths(lock.memo.get(&key), &paths, &needed)?;
    }
    lock.memo.insert(key.clone(), memo_entry(paths.clone()));
    if let Some(persistent) = &persistent {
        save_workspace_state(
            &persistent.2,
            &WorkspaceState {
                step_keys: step_keys.clone(),
            },
        )?;
    }
    let view = materialize_view(&paths)?;
    eprintln!(
        "BUILDER {builder_name} memo miss {} -> {view}",
        short_key(&key)
    );
    Ok(view)
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
                BuildStep::Fetch { command, .. } | BuildStep::Run { command, .. } => {
                    for binder in template_binders(command) {
                        add(binder, ".", None);
                    }
                }
            }
        }
    }
    for fetch in cixfile.fetches.values() {
        for binder in template_binders(&fetch.command) {
            add(binder, ".", None);
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

fn install_declared_expectations(
    builder_name: &str,
    builder: &Builder,
    commands: &[String],
    lock: &mut LockFile,
) {
    let mut command_index = 0;
    for (index, step) in builder.steps.iter().enumerate() {
        match step {
            BuildStep::Env { .. } => {}
            BuildStep::Fetch { expected, .. } => {
                let command = &commands[command_index];
                if let Some(expected) = expected {
                    let id = format!("builder:{builder_name}:{}", fetch_id(index, command));
                    if lock.fetches.get(&id).map(|pin| &pin.nar_hash) != Some(expected) {
                        lock.fetches
                            .insert(id, FetchPin::expected(expected.clone()));
                    }
                }
                command_index += 1;
            }
            BuildStep::Run { .. } => command_index += 1,
            BuildStep::Copy(_) => {}
        }
    }
}

fn builder_chain_keys(
    builder_name: &str,
    builder: &Builder,
    context: &BuildContext,
    offered_closure: &BTreeSet<String>,
    environment: &BTreeMap<String, String>,
    lock: &LockFile,
) -> Result<Option<Vec<String>>> {
    let mut environment = environment.clone();
    let mut predecessor = hex_hash(format!("BUILDER\0{builder_name}").as_bytes());
    let mut keys = Vec::with_capacity(builder.steps.len());
    let mut command_index = 0;
    let mut copy_index = 0;
    for (index, step) in builder.steps.iter().enumerate() {
        let (kind, arguments, sources, fetch_pin) = match step {
            BuildStep::Env { name, value, .. } => {
                environment.insert(name.clone(), value.clone());
                ("ENV", format!("{name}={value}"), Vec::new(), None)
            }
            BuildStep::Copy(copy) => {
                let source = &context.copies[copy_index];
                copy_index += 1;
                (
                    "COPY",
                    format!("{}\0{}", copy.source, copy.dst),
                    vec![nar_hash(Path::new(source))
                        .with_context(|| format!("hashing declared COPY source {source}"))?],
                    None,
                )
            }
            BuildStep::Fetch { .. } => {
                let command = &context.commands[command_index];
                command_index += 1;
                let id = format!("builder:{builder_name}:{}", fetch_id(index, command));
                let Some(pin) = lock.fetches.get(&id) else {
                    return Ok(None);
                };
                ("FETCH", command.clone(), Vec::new(), Some(pin.key()))
            }
            BuildStep::Run { .. } => {
                let command = &context.commands[command_index];
                command_index += 1;
                ("RUN", command.clone(), Vec::new(), None)
            }
        };
        predecessor = step_key(StepKeyRequest {
            kind,
            arguments: &arguments,
            offered_closure,
            ordered_imports: &context.imports,
            predecessor: &predecessor,
            declared_sources: &sources,
            environment: &environment,
            fetch_pin,
        })?;
        keys.push(predecessor.clone());
    }
    Ok(Some(keys))
}

fn top_fetch_chain_key(
    command: &str,
    offered_closure: &BTreeSet<String>,
    environment: &BTreeMap<String, String>,
    pin: &str,
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
    })
}

fn step_key(request: StepKeyRequest<'_>) -> Result<String> {
    Ok(hex_hash(&serde_json::to_vec(&(
        CODEGEN_FINGERPRINT,
        SANDBOX_SKELETON,
        request,
    ))?))
}

fn memo_has_paths(
    entry: Option<&MemoEntry>,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<bool> {
    let Some(entry) = entry else {
        return Ok(false);
    };
    if entry.legacy_store_path.is_some()
        || needed.keys().any(|path| !entry.paths.contains_key(path))
    {
        return Ok(false);
    }
    for path in needed.keys() {
        let consumed = &entry.paths[path];
        if !ensure_store_path(&consumed.store_path)?
            || nar_hash(Path::new(&consumed.store_path))? != consumed.nar_hash
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn store_consumed_paths(
    workspace: &Path,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<BTreeMap<String, ConsumedPath>> {
    let mut paths = BTreeMap::new();
    for path in needed.keys() {
        let source = if path == "." {
            workspace.to_owned()
        } else {
            workspace.join(path)
        };
        if !source.exists() && fs::symlink_metadata(&source).is_err() {
            bail!("consumed builder path {path:?} does not exist");
        }
        let nar_hash = nar_hash(&source)?;
        let store_path = add_store_object(&source, "cix-build-consumed")?;
        paths.insert(
            path.clone(),
            ConsumedPath {
                nar_hash,
                store_path,
            },
        );
    }
    Ok(paths)
}

fn compare_cold_paths(
    warm: Option<&MemoEntry>,
    cold: &BTreeMap<String, ConsumedPath>,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<()> {
    let Some(warm) = warm.filter(|entry| entry.legacy_store_path.is_none()) else {
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
    MemoEntry {
        paths,
        legacy_output_nar_hash: None,
        legacy_store_path: None,
        legacy_wall_ms: None,
    }
}

fn materialize_view(paths: &BTreeMap<String, ConsumedPath>) -> Result<String> {
    if let Some(whole) = paths.get(".") {
        return Ok(whole.store_path.clone());
    }
    let view = tempfile::Builder::new()
        .prefix("cix-build-view-")
        .tempdir()
        .context("creating consumed-path view")?;
    for (path, consumed) in paths {
        copy_node(Path::new(&consumed.store_path), &view.path().join(path))?;
    }
    add_store_object(view.path(), "cix-build-view")
}

fn add_store_object(path: &Path, name: &str) -> Result<String> {
    let path = path
        .to_str()
        .context("store input path is not valid UTF-8")?;
    cix_common::nix(&["store", "add", "--mode", "nar", "--name", name, path])?
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .context("nix store add did not return a store path")
}

fn workspace_paths(directory: &Path, builder: &str) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let base = if let Some(path) = std::env::var_os("CIX_BUILD_WORKSPACE_DIR") {
        PathBuf::from(path)
    } else if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path).join("cix/workspaces")
    } else {
        PathBuf::from(
            std::env::var_os("HOME")
                .context("HOME is unset; set CIX_BUILD_WORKSPACE_DIR for Cixfile workspaces")?,
        )
        .join(".cache/cix/workspaces")
    };
    let identity = workspace_identity(directory, builder);
    let root = base.join(identity);
    let work = root.join("work");
    let staged = root.join("staged");
    let state = root.join("state.json");
    fs::create_dir_all(&work)
        .with_context(|| format!("creating persistent builder workspace {}", work.display()))?;
    fs::create_dir_all(&staged)
        .with_context(|| format!("creating builder staging records {}", staged.display()))?;
    Ok((work, staged, state))
}

fn load_workspace_state(path: &Path) -> Option<WorkspaceState> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn save_workspace_state(path: &Path, state: &WorkspaceState) -> Result<()> {
    let temporary = path.with_extension("json.next");
    fs::write(&temporary, serde_json::to_vec(state)?)
        .with_context(|| format!("writing builder workspace state {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("replacing builder workspace state {}", path.display()))
}

fn replace_workspace_tree(source: &Path, destination: &Path) -> Result<()> {
    remove_path_if_present(destination)?;
    fs::create_dir_all(destination)?;
    copy_tree(source, destination)
}

fn restore_snapshot(snapshot: &Path, destination: &Path) -> Result<()> {
    if !ensure_store_path(
        snapshot
            .to_str()
            .context("FETCH snapshot path is not UTF-8")?,
    )? {
        bail!(
            "pinned FETCH snapshot {} is unavailable locally; run --update-lock to refresh it",
            snapshot.display()
        );
    }
    replace_workspace_tree(snapshot, destination)?;
    make_writable(destination)
}

fn replay_fetch_snapshot(directory: &Path, name: &str, pin: &FetchPin) -> Result<String> {
    if let Some(snapshot) = pin.store_path.as_deref() {
        if ensure_store_path(snapshot)? {
            return Ok(snapshot.to_owned());
        }
    }
    let receipt = fetch_snapshot_receipt(directory, name, pin)?;
    let snapshot = fs::read_to_string(&receipt)
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|path| !path.is_empty())
        .filter(|path| ensure_store_path(path).unwrap_or(false));
    snapshot.with_context(|| {
        format!(
            "FETCH {name} has no locally cached replay snapshot at {}; run a non-cold build first (--cold never refetches)",
            receipt.display()
        )
    })
}

fn cache_fetch_snapshot(
    directory: &Path,
    name: &str,
    pin: &FetchPin,
    snapshot: &str,
) -> Result<()> {
    let receipt = fetch_snapshot_receipt(directory, name, pin)?;
    let parent = receipt
        .parent()
        .expect("fetch snapshot receipt has a parent");
    fs::create_dir_all(parent)
        .with_context(|| format!("creating FETCH snapshot cache {}", parent.display()))?;
    fs::write(&receipt, format!("{snapshot}\n"))
        .with_context(|| format!("recording FETCH snapshot cache {}", receipt.display()))
}

fn fetch_snapshot_receipt(directory: &Path, name: &str, pin: &FetchPin) -> Result<PathBuf> {
    let base = if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path)
    } else {
        PathBuf::from(
            std::env::var_os("HOME")
                .context("HOME is unset; set XDG_CACHE_HOME for FETCH replay snapshots")?,
        )
        .join(".cache")
    };
    let directory = directory.canonicalize().with_context(|| {
        format!(
            "resolving Cixfile directory for FETCH snapshot cache {}",
            directory.display()
        )
    })?;
    let key = hex_hash(format!("{}\0{name}\0{}", directory.display(), pin.key()).as_bytes());
    Ok(base.join("cix/fetch-snapshots").join(key))
}

fn copied_snapshot(source: &Path) -> Result<FetchProbe> {
    let snapshot = tempfile::Builder::new()
        .prefix("cix-fetch-probe-")
        .tempdir()
        .context("creating FETCH probe snapshot")?;
    copy_tree(source, snapshot.path())?;
    Ok(FetchProbe {
        temporary: Some(snapshot),
    })
}

fn cleanup_fetch_probe(snapshot: tempfile::TempDir) -> Result<()> {
    let writable = make_writable(snapshot.path())
        .context("making FETCH probe snapshot writable before removal");
    let removed = snapshot.close().context("removing FETCH probe snapshot");
    match (writable, removed) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(writable), Err(removed)) => Err(writable.context(format!(
            "also failed to remove FETCH probe snapshot: {removed:#}"
        ))),
    }
}

fn volatile_paths(first: &Path, second: &Path) -> Result<BTreeMap<String, VolatilePath>> {
    let mut first_nodes = BTreeMap::new();
    let mut second_nodes = BTreeMap::new();
    collect_files(first, Path::new(""), &mut first_nodes)?;
    collect_files(second, Path::new(""), &mut second_nodes)?;
    let names = first_nodes
        .keys()
        .chain(second_nodes.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut volatile = BTreeMap::new();
    for name in names {
        let before = first_nodes.get(&name);
        let after = second_nodes.get(&name);
        if before.map(|node| &node.0) != after.map(|node| &node.0) {
            volatile.insert(
                name,
                VolatilePath {
                    first_size: before.map_or(0, |node| node.1),
                    second_size: after.map_or(0, |node| node.1),
                },
            );
        }
    }
    Ok(volatile)
}

fn consumed_volatile_paths(
    observed: BTreeMap<String, VolatilePath>,
    needed: &BTreeMap<String, NeededPath>,
) -> BTreeMap<String, VolatilePath> {
    observed
        .into_iter()
        .filter(|(path, _)| {
            needed.keys().any(|needed_path| {
                needed_path == "."
                    || path == needed_path
                    || path
                        .strip_prefix(needed_path)
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
        })
        .collect()
}

fn report_volatile(name: &str, volatile: &BTreeMap<String, VolatilePath>) {
    if volatile.is_empty() {
        eprintln!("FETCH {name} update probe: two outputs were identical");
        return;
    }
    eprintln!("FETCH {name} update probe found volatile files:");
    for (path, sizes) in volatile {
        eprintln!(
            "  {path} ({} B -> {} B)",
            sizes.first_size, sizes.second_size
        );
    }
}

fn collect_files(
    root: &Path,
    relative: &Path,
    files: &mut BTreeMap<String, (String, u64)>,
) -> Result<()> {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory)
        .with_context(|| format!("reading FETCH probe tree {}", directory.display()))?
    {
        let entry = entry?;
        let name = relative.join(entry.file_name());
        let path = root.join(&name);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            collect_files(root, &name, files)?;
        } else {
            files.insert(
                name.to_string_lossy().into_owned(),
                (file_fingerprint(&path, &metadata)?, metadata.len()),
            );
        }
    }
    Ok(())
}

fn file_fingerprint(path: &Path, metadata: &fs::Metadata) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(metadata.permissions().mode().to_le_bytes());
    if metadata.file_type().is_symlink() {
        hasher.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else {
        let mut file = fs::File::open(path)?;
        io::copy(&mut file, &mut hasher)?;
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn workspace_identity(directory: &Path, builder: &str) -> String {
    hex_hash(format!("{}\0{builder}", directory.to_string_lossy()).as_bytes())
}

fn stage_input(source: &Path, dst: &str, workspace: &Path, baseline: &Path) -> Result<()> {
    let first_application = !baseline.exists();
    let next = baseline.with_extension("next");
    remove_path_if_present(&next)?;
    fs::create_dir_all(&next)?;
    copy_input(source, dst, &next)?;
    sync_directories(
        baseline.exists().then_some(baseline),
        &next,
        workspace,
        first_application,
    )?;
    make_writable(workspace)?;
    remove_path_if_present(baseline)?;
    fs::rename(&next, baseline).with_context(|| {
        format!(
            "replacing staged-input record {} with {}",
            baseline.display(),
            next.display()
        )
    })?;
    make_writable(baseline)?;
    Ok(())
}

fn sync_directories(
    old: Option<&Path>,
    new: &Path,
    workspace: &Path,
    first_application: bool,
) -> Result<()> {
    let mut names = BTreeSet::new();
    if let Some(old) = old {
        for entry in fs::read_dir(old)? {
            names.insert(entry?.file_name());
        }
    }
    for entry in fs::read_dir(new)? {
        names.insert(entry?.file_name());
    }
    for name in names {
        sync_node(
            old.map(|root| root.join(&name)).as_deref(),
            Some(&new.join(&name)),
            &workspace.join(&name),
            first_application,
        )?;
    }
    Ok(())
}

fn sync_node(
    old: Option<&Path>,
    new: Option<&Path>,
    workspace: &Path,
    first_application: bool,
) -> Result<()> {
    let old = old.filter(|path| fs::symlink_metadata(path).is_ok());
    let new = new.filter(|path| fs::symlink_metadata(path).is_ok());
    let work_exists = fs::symlink_metadata(workspace).is_ok();
    match (old, new, work_exists) {
        (None, Some(new), false) => copy_node(new, workspace),
        (None, Some(new), true) if first_application && new.is_dir() && workspace.is_dir() => {
            sync_directories(None, new, workspace, true)
        }
        (None, Some(new), true) if first_application => {
            remove_path_if_present(workspace)?;
            copy_node(new, workspace)
        }
        (None, Some(_), true) | (None, None, _) | (Some(_), _, false) => Ok(()),
        (Some(old), Some(new), true) if old.is_dir() && new.is_dir() && workspace.is_dir() => {
            sync_directories(Some(old), new, workspace, first_application)
        }
        (Some(old), None, true) if old.is_dir() && workspace.is_dir() => {
            let empty = tempfile::tempdir()?;
            sync_directories(Some(old), empty.path(), workspace, first_application)?;
            if fs::read_dir(workspace)?.next().is_none() {
                fs::remove_dir(workspace)?;
            }
            Ok(())
        }
        (Some(old), new, true) if nodes_equal(old, workspace)? => {
            remove_path_if_present(workspace)?;
            if let Some(new) = new {
                copy_node(new, workspace)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn nodes_equal(left: &Path, right: &Path) -> Result<bool> {
    let left_meta = fs::symlink_metadata(left)?;
    let right_meta = fs::symlink_metadata(right)?;
    if left_meta.file_type().is_symlink() || right_meta.file_type().is_symlink() {
        return Ok(left_meta.file_type().is_symlink()
            && right_meta.file_type().is_symlink()
            && fs::read_link(left)? == fs::read_link(right)?);
    }
    if left_meta.is_file() || right_meta.is_file() {
        return Ok(left_meta.is_file()
            && right_meta.is_file()
            && (left_meta.permissions().mode() & 0o111)
                == (right_meta.permissions().mode() & 0o111)
            && fs::read(left)? == fs::read(right)?);
    }
    if !left_meta.is_dir() || !right_meta.is_dir() {
        return Ok(false);
    }
    let mut left_names = fs::read_dir(left)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut right_names = fs::read_dir(right)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<Vec<_>>>()?;
    left_names.sort();
    right_names.sort();
    if left_names != right_names {
        return Ok(false);
    }
    for name in left_names {
        if !nodes_equal(&left.join(&name), &right.join(&name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn copy_node(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if metadata.file_type().is_symlink() {
        symlink(fs::read_link(source)?, destination)?;
    } else if metadata.is_dir() {
        fs::create_dir(destination)?;
        copy_tree(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
    } else {
        bail!("unsupported special file {}", source.display());
    }
    Ok(())
}

fn make_writable(path: &Path) -> Result<()> {
    let root_metadata = fs::symlink_metadata(path)?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.is_dir() {
            make_writable(&entry_path)?;
        }
        if !metadata.file_type().is_symlink() {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0o200);
            fs::set_permissions(&entry_path, permissions)?;
        }
    }
    let mut permissions = root_metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn ensure_store_path(path: &str) -> Result<bool> {
    if Path::new(path).exists() {
        return Ok(true);
    }
    let output = Command::new("nix-store")
        .args(["--realise", path])
        .output()
        .with_context(|| format!("asking substituters for memo output {path}"))?;
    Ok(output.status.success() && Path::new(path).exists())
}

fn resolve_builder_context(
    cixfile: &Cixfile,
    builder: &str,
    directory: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<BuildContext> {
    let expression =
        generate_builder_context_nix(cixfile, builder, directory, lock, system, snapshots)?;
    eval_context(&expression)
}

fn resolve_fetch_context(
    cixfile: &Cixfile,
    fetch: &str,
    directory: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<BuildContext> {
    let expression =
        generate_fetch_context_nix(cixfile, fetch, directory, lock, system, snapshots)?;
    eval_context(&expression)
}

fn eval_context(expression: &str) -> Result<BuildContext> {
    let raw = cix_common::nix(&["eval", "--impure", "--json", "--expr", expression])
        .context("resolving RUN/FETCH build context from locked FROM inputs")?;
    serde_json::from_str(&raw).context("parsing resolved RUN/FETCH build context")
}

fn realize_builder_offers(
    cixfile: &Cixfile,
    builder: &str,
    directory: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<()> {
    let expression =
        generate_builder_offer_nix(cixfile, builder, directory, lock, system, snapshots)?;
    realize_offers(&expression)
}

fn realize_fetch_offers(
    cixfile: &Cixfile,
    fetch: &str,
    directory: &Path,
    lock: &LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
) -> Result<()> {
    let expression = generate_fetch_offer_nix(cixfile, fetch, directory, lock, system, snapshots)?;
    realize_offers(&expression)
}

fn realize_offers(expression: &str) -> Result<()> {
    cix_common::nix(&[
        "build",
        "--impure",
        "--no-link",
        "--print-out-paths",
        "--expr",
        expression,
    ])
    .context("realizing offered RUN/FETCH closure")?;
    Ok(())
}

fn query_closure(offers: &[String]) -> Result<BTreeSet<String>> {
    let output = Command::new("nix-store")
        .args(["--query", "--requisites"])
        .args(offers)
        .output()
        .context("executing nix-store to resolve offered closure")?;
    if !output.status.success() {
        bail!(
            "nix-store --query --requisites failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout).context("nix-store returned non-UTF-8 paths")?;
    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn find_shell(paths: &[String]) -> Result<String> {
    paths
        .iter()
        .map(|package| Path::new(package).join("bin/bash"))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
        .context("RUN/FETCH requires bash in an IMPORTed package")
}

#[allow(clippy::too_many_arguments)]
fn run_sandbox(
    workdir: &Path,
    _shell: &str,
    command: &str,
    environment: &BTreeMap<String, String>,
    export_prelude: &BTreeMap<String, String>,
    offered_closure: &BTreeSet<String>,
    imports: &[String],
    run_network: Option<RunNetwork>,
) -> Result<()> {
    let import_union = prepare_import_union(imports, run_network.is_none())?;
    let env_is_missing = !import_union.path().join("bin/env").is_file();
    let mut process = Command::new("bwrap");
    process.args([
        "--die-with-parent",
        "--new-session",
        "--unshare-user",
        "--uid",
        "0",
        "--gid",
        "0",
        "--unshare-pid",
        "--unshare-uts",
        "--unshare-ipc",
        "--unshare-cgroup",
    ]);
    if run_network == Some(RunNetwork::Namespace) {
        process.arg("--unshare-net");
    }
    let _seccomp_filter = if run_network == Some(RunNetwork::SocketFilter) {
        Some(seccomp::attach_socket_filter(&mut process)?)
    } else {
        None
    };
    process.args(["--dir", "/nix", "--dir", "/nix/store"]);
    process.args(["--dir", "/usr", "--dir", "/usr/bin"]);
    process.args(["--symlink", "/bin/env", "/usr/bin/env"]);
    for path in offered_closure {
        process.args(["--ro-bind", path, path]);
    }
    for subtree in ["bin", "etc", "share"] {
        let source = import_union.path().join(subtree);
        if source.is_dir() {
            process
                .arg("--ro-bind")
                .arg(&source)
                .arg(Path::new("/").join(subtree));
        }
    }
    process.args(["--bind"]).arg(workdir).arg("/work").args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--chdir",
        "/work",
        "--clearenv",
    ]);
    if !import_union.path().join("etc").is_dir() {
        process.args(["--dir", "/etc"]);
    }
    for (name, value) in environment {
        process.args(["--setenv", name, value]);
    }
    let exports = export_prelude
        .iter()
        .map(|(name, value)| format!("export {name}={value};"))
        .collect::<String>();
    let shell_program = format!("umask 022; {exports}eval \"$1\"");
    let output = process
        .arg("/bin/bash")
        .args(["-c", &shell_program, "cix-build", command])
        .output()
        .context(
            "starting bubblewrap sandbox; this host may restrict unprivileged user namespaces",
        )?;
    io::stderr().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        let mut failure = sandbox_failure(output.status, run_network);
        if env_is_missing {
            failure.push_str(
                "\nhint: /usr/bin/env is a fixed alias to /bin/env; IMPORT ${pkgs.coreutils} or another package that supplies env",
            );
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            failure.push_str("\ncommand stderr:\n");
            failure.push_str(stderr.trim());
        }
        bail!("{failure}");
    }
    Ok(())
}

fn prepare_import_union(
    imports: &[String],
    include_network_configuration: bool,
) -> Result<tempfile::TempDir> {
    let union = tempfile::Builder::new()
        .prefix("cix-import-union-")
        .tempdir()
        .context("creating IMPORT package union")?;
    for package in imports {
        let package = Path::new(package);
        if !package.is_absolute() {
            bail!(
                "IMPORT resolved to non-absolute package path {}",
                package.display()
            );
        }
        for subtree in ["bin", "etc", "share"] {
            let source = package.join(subtree);
            if !source.is_dir() {
                continue;
            }
            let destination = union.path().join(subtree);
            fs::create_dir_all(&destination)?;
            merge_import_directory(&source, &destination)?;
        }
    }
    if include_network_configuration {
        let etc = union.path().join("etc");
        fs::create_dir_all(&etc)?;
        for source in ["/etc/hosts", "/etc/nsswitch.conf", "/etc/resolv.conf"] {
            let source = Path::new(source);
            if !source.is_file() {
                continue;
            }
            let destination = etc.join(source.file_name().expect("network file has a name"));
            remove_path_if_present(&destination)?;
            fs::copy(source, &destination)?;
        }
    }
    Ok(union)
}

fn merge_import_directory(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("reading IMPORT subtree {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let source_metadata = fs::symlink_metadata(&source_path)?;
        let destination_metadata = fs::symlink_metadata(&destination_path).ok();
        if let Some(destination_metadata) = destination_metadata {
            if source_metadata.is_dir()
                && !source_metadata.file_type().is_symlink()
                && destination_metadata.is_dir()
                && !destination_metadata.file_type().is_symlink()
            {
                merge_import_directory(&source_path, &destination_path)?;
            }
            continue;
        }
        if source_metadata.is_dir() && !source_metadata.file_type().is_symlink() {
            fs::create_dir(&destination_path)?;
            merge_import_directory(&source_path, &destination_path)?;
        } else {
            symlink(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn remove_path_if_present(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        make_writable(path)?;
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("removing {}", path.display()))
}

fn probe_run_network(shell: &str) -> Result<RunNetwork> {
    let output = Command::new("bwrap")
        .args([
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--uid",
            "0",
            "--gid",
            "0",
            "--unshare-net",
            "--ro-bind",
            "/",
            "/",
            "--",
            shell,
            "-c",
            "true",
        ])
        .output()
        .context(
            "probing bubblewrap network isolation; this host may restrict unprivileged user namespaces",
        )?;
    Ok(if output.status.success() {
        RunNetwork::Namespace
    } else {
        RunNetwork::SocketFilter
    })
}

fn sandbox_failure(status: impl std::fmt::Display, run_network: Option<RunNetwork>) -> String {
    let mut message = format!(
        "bubblewrap sandbox or command exited {status}; sandboxing was not weakened (enable unprivileged user namespaces if bwrap reported a namespace permission error)"
    );
    if run_network == Some(RunNetwork::SocketFilter) {
        message.push_str(
            "\nhint: this RUN used the socket-filter fallback because the host rejected bubblewrap's network namespace (often an AppArmor restriction); localhost networking (127.0.0.1) was unavailable",
        );
    }
    message
}

fn copy_input(source: &Path, dst: &str, workdir: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("reading COPY source {}", source.display()))?;
    let destination = if dst == "." && !metadata.is_dir() {
        workdir.join(
            source
                .file_name()
                .context("COPY source has no final path component")?,
        )
    } else {
        workdir.join(dst)
    };
    if metadata.is_dir() {
        if dst == "." {
            copy_tree(source, workdir)?;
        } else {
            fs::create_dir(&destination)
                .with_context(|| format!("creating COPY directory {}", destination.display()))?;
            copy_tree(source, &destination)?;
            fs::set_permissions(&destination, metadata.permissions())?;
        }
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating COPY destination {}", parent.display()))?;
    }
    if metadata.file_type().is_symlink() {
        symlink(fs::read_link(source)?, &destination).with_context(|| {
            format!(
                "copying symlink {} to build workdir destination {}",
                source.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }
    if !metadata.is_file() {
        bail!("COPY source {} is a special file", source.display());
    }
    fs::copy(source, &destination).with_context(|| {
        format!(
            "copying {} to build workdir destination {}",
            source.display(),
            destination.display()
        )
    })?;
    fs::set_permissions(&destination, metadata.permissions())?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("reading snapshot directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            symlink(fs::read_link(&source_path)?, &destination_path)?;
        } else if metadata.is_dir() {
            fs::create_dir(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else {
            bail!(
                "unsupported special file in build snapshot: {}",
                source_path.display()
            );
        }
    }
    Ok(())
}

fn nar_hash(path: &Path) -> Result<String> {
    let path_text = path.to_str().context("path is not valid UTF-8")?;
    Ok(
        cix_common::nix(&["hash", "path", "--mode", "nar", path_text])?
            .trim()
            .to_owned(),
    )
}

fn fetch_path_hashes(
    workspace: &Path,
    needed: &BTreeMap<String, NeededPath>,
) -> Result<BTreeMap<String, String>> {
    let mut paths = BTreeMap::new();
    for path in needed.keys() {
        let source = if path == "." {
            workspace.to_owned()
        } else {
            workspace.join(path)
        };
        if !source.exists() && fs::symlink_metadata(&source).is_err() {
            bail!("FETCH-consumed path {path:?} does not exist");
        }
        paths.insert(path.clone(), nar_hash(&source)?);
    }
    Ok(paths)
}

#[allow(clippy::too_many_arguments)]
fn refresh_fetch_pin(
    previous: Option<&FetchPin>,
    expected: bool,
    force: bool,
    actual_paths: BTreeMap<String, String>,
    volatile: BTreeMap<String, VolatilePath>,
    name: &str,
) -> Result<FetchPin> {
    if expected {
        let mut pin = previous
            .cloned()
            .context("declared EXPECT pin was not installed")?;
        pin.store_path = None;
        if !volatile.is_empty() {
            pin.volatile = volatile;
        }
        return Ok(pin);
    }

    let mut pin = previous.cloned().unwrap_or_else(FetchPin::automatic);
    if !force && !pin.paths.is_empty() {
        for (path, pinned) in &pin.paths {
            let actual = actual_paths
                .get(path)
                .with_context(|| format!("FETCH pin's consumed path {path:?} disappeared"))?;
            if actual != pinned {
                bail!(
                    "FETCH consumed-path mismatch at {path:?}: lock pins {pinned}, fetched {actual}; rerun with --update-lock to accept the new output"
                );
            }
        }
        for path in actual_paths
            .keys()
            .filter(|path| !pin.paths.contains_key(*path))
        {
            eprintln!(
                "FETCH {name} consumed a newly observed path {path:?}; recording a fresh pin entry"
            );
        }
    }
    pin.nar_hash.clear();
    pin.paths = actual_paths;
    pin.store_path = None;
    if force {
        pin.volatile = volatile;
    }
    Ok(pin)
}

fn fetch_id(index: usize, command: &str) -> String {
    format!("{index}-{}", short_key(&hex_hash(command.as_bytes())))
}

fn verify_fetch_pin(pin: Option<&FetchPin>, actual: Option<&str>) -> Result<()> {
    if let Some(pin) = pin {
        if pin.nar_hash.is_empty() && actual.is_none() {
            return Ok(());
        }
        let actual = actual.context("FETCH pin needs fetched bytes for whole-tree verification")?;
        if pin.nar_hash != actual {
            bail!(
                "FETCH hash mismatch: lock pins {}, fetched {}; rerun with --update-lock to accept the new output",
                pin.nar_hash,
                actual
            );
        }
    }
    Ok(())
}

fn verify_fetch_hash(
    expected: Option<&str>,
    pin: Option<&FetchPin>,
    actual: Option<&str>,
) -> Result<()> {
    if let Some(expected) = expected {
        if let Some(actual) = actual {
            if expected != actual {
                bail!("FETCH EXPECT hash mismatch: declared {expected}, fetched {actual}");
            }
        } else if pin.is_none_or(|pin| pin.nar_hash != expected) {
            bail!("FETCH EXPECT hash mismatch: declared {expected}, lock has no matching pin");
        }
        return Ok(());
    }
    verify_fetch_pin(pin, actual)
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn short_key(key: &str) -> &str {
    &key[..12.min(key.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closure(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn step_key_tracks_chain_inputs_without_workdir_bytes() {
        let environment = BTreeMap::from([("PATH".into(), "/nix/store/tool/bin".into())]);
        let base = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "cargo build",
            offered_closure: &closure(&["/nix/store/tool"]),
            ordered_imports: &[],
            predecessor: "previous-key",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
        })
        .unwrap();
        assert_eq!(
            base,
            step_key(StepKeyRequest {
                kind: "RUN",
                arguments: "cargo build",
                offered_closure: &closure(&["/nix/store/tool"]),
                ordered_imports: &[],
                predecessor: "previous-key",
                declared_sources: &[],
                environment: &environment,
                fetch_pin: None,
            })
            .unwrap()
        );
        assert_ne!(
            base,
            step_key(StepKeyRequest {
                kind: "RUN",
                arguments: "cargo test",
                offered_closure: &closure(&["/nix/store/tool"]),
                ordered_imports: &[],
                predecessor: "previous-key",
                declared_sources: &[],
                environment: &environment,
                fetch_pin: None,
            })
            .unwrap()
        );
        let changed_environment =
            BTreeMap::from([("PATH".into(), "/nix/store/other-tool/bin".into())]);
        assert_ne!(
            base,
            step_key(StepKeyRequest {
                kind: "RUN",
                arguments: "cargo build",
                offered_closure: &closure(&["/nix/store/tool"]),
                ordered_imports: &[],
                predecessor: "previous-key",
                declared_sources: &[],
                environment: &changed_environment,
                fetch_pin: None,
            })
            .unwrap()
        );
        assert_ne!(
            base,
            step_key(StepKeyRequest {
                kind: "RUN",
                arguments: "cargo build",
                offered_closure: &closure(&["/nix/store/new-tool"]),
                ordered_imports: &[],
                predecessor: "previous-key",
                declared_sources: &[],
                environment: &environment,
                fetch_pin: None,
            })
            .unwrap()
        );
        assert_ne!(
            base,
            step_key(StepKeyRequest {
                kind: "RUN",
                arguments: "cargo build",
                offered_closure: &closure(&["/nix/store/tool"]),
                ordered_imports: &[],
                predecessor: "changed-predecessor",
                declared_sources: &[],
                environment: &environment,
                fetch_pin: None,
            })
            .unwrap()
        );
    }

    #[test]
    fn copy_source_hash_and_fetch_pin_participate_in_chain_keys() {
        let environment = BTreeMap::new();
        let offered = closure(&["/nix/store/tool"]);
        let before = step_key(StepKeyRequest {
            kind: "COPY",
            arguments: "COPY src .",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            predecessor: "previous",
            declared_sources: &["sha256-source-one".into()],
            environment: &environment,
            fetch_pin: None,
        })
        .unwrap();
        let after = step_key(StepKeyRequest {
            kind: "COPY",
            arguments: "COPY src .",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            predecessor: "previous",
            declared_sources: &["sha256-source-two".into()],
            environment: &environment,
            fetch_pin: None,
        })
        .unwrap();
        assert_ne!(before, after);
        assert_ne!(
            top_fetch_chain_key("fetch", &offered, &environment, "sha256-one").unwrap(),
            top_fetch_chain_key("fetch", &offered, &environment, "sha256-two").unwrap()
        );
    }

    #[test]
    fn ordered_imports_participate_in_chain_keys() {
        let environment = BTreeMap::from([("PATH".into(), "/bin".into())]);
        let offered = closure(&["/nix/store/one", "/nix/store/two"]);
        let one_first = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "tool",
            offered_closure: &offered,
            ordered_imports: &["/nix/store/one".into(), "/nix/store/two".into()],
            predecessor: "previous",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
        })
        .unwrap();
        let two_first = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "tool",
            offered_closure: &offered,
            ordered_imports: &["/nix/store/two".into(), "/nix/store/one".into()],
            predecessor: "previous",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
        })
        .unwrap();
        assert_ne!(one_first, two_first);
    }

    #[test]
    fn import_union_merges_subtrees_and_preserves_earlier_collisions() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        for package in [&first, &second] {
            fs::create_dir_all(package.join("bin")).unwrap();
            fs::create_dir_all(package.join("etc/tool")).unwrap();
            fs::create_dir_all(package.join("share/tool")).unwrap();
        }
        fs::write(first.join("bin/collision"), "first").unwrap();
        fs::write(second.join("bin/collision"), "second").unwrap();
        fs::write(first.join("etc/tool/first"), "first").unwrap();
        fs::write(second.join("etc/tool/second"), "second").unwrap();
        fs::write(second.join("share/tool/data"), "shared").unwrap();

        let union = prepare_import_union(
            &[
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(union.path().join("bin/collision")).unwrap(),
            "first"
        );
        assert_eq!(
            fs::read_to_string(union.path().join("etc/tool/first")).unwrap(),
            "first"
        );
        assert_eq!(
            fs::read_to_string(union.path().join("etc/tool/second")).unwrap(),
            "second"
        );
        assert_eq!(
            fs::read_to_string(union.path().join("share/tool/data")).unwrap(),
            "shared"
        );
    }

    #[test]
    fn cold_mismatch_names_the_exact_consuming_copy() {
        let warm = memo_entry(BTreeMap::from([(
            "target/release/app".into(),
            ConsumedPath {
                nar_hash: "sha256-warm".into(),
                store_path: "/nix/store/warm".into(),
            },
        )]));
        let cold = BTreeMap::from([(
            "target/release/app".into(),
            ConsumedPath {
                nar_hash: "sha256-cold".into(),
                store_path: "/nix/store/cold".into(),
            },
        )]);
        let needed = BTreeMap::from([(
            "target/release/app".into(),
            NeededPath {
                attributions: vec![Attribution {
                    binder: "build".into(),
                    path: "target/release/app".into(),
                    line: 17,
                }],
            },
        )]);
        let error = compare_cold_paths(Some(&warm), &cold, &needed)
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "COPY ${build}/target/release/app (line 17) differs between warm and cold"
        );
    }

    #[test]
    fn first_staging_overrides_prior_step_output_then_preserves_upper_writes() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("work");
        let baseline = root.path().join("staged/step");
        let source = root.path().join("source");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("value"), "earlier command").unwrap();
        fs::write(&source, "declared v1").unwrap();

        stage_input(&source, "value", &workspace, &baseline).unwrap();
        assert_eq!(
            fs::read_to_string(workspace.join("value")).unwrap(),
            "declared v1"
        );

        fs::write(workspace.join("value"), "later command").unwrap();
        fs::write(&source, "declared v2").unwrap();
        stage_input(&source, "value", &workspace, &baseline).unwrap();
        assert_eq!(
            fs::read_to_string(workspace.join("value")).unwrap(),
            "later command"
        );
    }

    #[test]
    fn fetch_pin_mismatch_is_loud_and_names_update_lock() {
        let error = verify_fetch_pin(
            Some(&FetchPin::expected("sha256-old".into())),
            Some("sha256-new"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("hash mismatch"), "{error}");
        assert!(error.contains("--update-lock"), "{error}");
    }

    #[test]
    fn volatile_facts_follow_only_consumed_path_boundaries() {
        let observed = BTreeMap::from([
            (
                ".npm/_logs/timestamped-debug.log".into(),
                VolatilePath {
                    first_size: 1,
                    second_size: 2,
                },
            ),
            (
                "node_modules/pkg/index.js".into(),
                VolatilePath {
                    first_size: 3,
                    second_size: 4,
                },
            ),
            (
                "result".into(),
                VolatilePath {
                    first_size: 5,
                    second_size: 6,
                },
            ),
        ]);
        let needed = BTreeMap::from([
            ("node_modules".into(), NeededPath::default()),
            ("result".into(), NeededPath::default()),
        ]);

        assert_eq!(
            consumed_volatile_paths(observed, &needed)
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["node_modules/pkg/index.js", "result"]
        );
    }

    #[test]
    fn persistent_workspace_identity_includes_builder_name() {
        let directory = Path::new("/work/project");
        assert_ne!(
            workspace_identity(directory, "frontend"),
            workspace_identity(directory, "backend")
        );
        assert_eq!(
            workspace_identity(directory, "frontend"),
            workspace_identity(directory, "frontend")
        );
    }

    #[test]
    fn socket_filter_failure_adds_localhost_hint() {
        let error = sandbox_failure("exit status: 1", Some(RunNetwork::SocketFilter));
        assert!(error.contains("sandboxing was not weakened"), "{error}");
        assert!(error.contains("socket-filter fallback"), "{error}");
        assert!(
            error.contains("localhost networking (127.0.0.1) was unavailable"),
            "{error}"
        );
        assert_eq!(error.lines().count(), 2, "{error}");

        let preferred = sandbox_failure("exit status: 1", Some(RunNetwork::Namespace));
        assert!(!preferred.contains("localhost"), "{preferred}");
    }

    #[test]
    fn socket_filter_is_accepted_by_bubblewrap() {
        let shell = fs::read_dir("/nix/store")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("bin/bash"))
            .find(|candidate| candidate.is_file())
            .expect("the Nix test host provides bash");
        let offer = shell
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let offered_closure = query_closure(std::slice::from_ref(&offer)).unwrap();
        let work = tempfile::tempdir().unwrap();

        run_sandbox(
            work.path(),
            shell.to_str().unwrap(),
            "printf fallback-ok > result",
            &BTreeMap::new(),
            &BTreeMap::new(),
            &offered_closure,
            &[offer],
            Some(RunNetwork::SocketFilter),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(work.path().join("result")).unwrap(),
            "fallback-ok"
        );
    }
}
