use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::codegen::{
    generate_builder_context_nix, generate_builder_dev_env_nix, generate_builder_offer_nix,
    generate_fetch_context_nix, generate_fetch_offer_nix,
};
use crate::fetch::{CredentialMount, HostCredentials};
use crate::fhs;
use crate::seccomp;
use crate::trace;
use crate::{
    BuildStep, Builder, Cixfile, ConsumedPath, Copy, DevEnvironment, Fetch, FetchPin, LockFile,
    MemoEntry, StepChange, StepMemo, Template, TemplatePart, VolatilePath,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildContext {
    offers: Vec<String>,
    imports: Vec<String>,
    commands: Vec<String>,
    copies: Vec<String>,
    environment: BTreeMap<String, String>,
    #[serde(rename = "universeIdentities")]
    universe_identities: BTreeMap<String, String>,
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
    universe_identities: &'a [String],
}

#[derive(Serialize)]
struct StepMemoKeyRequest<'a> {
    builder: &'a str,
    index: usize,
    kind: &'a str,
    directive: &'a str,
    arguments: &'a str,
    offered_closure: &'a BTreeSet<String>,
    ordered_imports: &'a [String],
    environment: &'a BTreeMap<String, String>,
    universe_identities: &'a [String],
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

// Bump whenever the fixed bubblewrap filesystem skeleton changes: memoized
// commands must not be reused across a different execution environment.
const SANDBOX_SKELETON: &str = fhs::SKELETON_FINGERPRINT;
// Bump this when codegen-relevant Cixfile semantics change without a package
// version bump.  It keeps memo keys isolated across concurrently-built checkouts.
const CODEGEN_FINGERPRINT: &str = crate::BUILDER_FINGERPRINT;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutedStep {
    pub name: String,
    pub kind: String,
    pub executed: bool,
}

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
    /// Step memos whose outputs this workspace already holds, by memo owner →
    /// memo key. Trusted the same way `step_keys` is for the rerun prefix: a
    /// matching entry lets a memo hit skip re-materializing outputs that the
    /// previous build in this workspace already applied or produced.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    materialized_memos: BTreeMap<String, String>,
    /// Local metadata fingerprints for FETCH output roots. These are never a
    /// substitute for the lockfile's content hashes: a mismatch falls back to
    /// hashing the recorded output before a self-observation is accepted.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    memo_output_fingerprints: BTreeMap<String, BTreeMap<String, String>>,
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

/* FETCH credential and consent types live in fetch.rs. */
/*
struct CredentialsFile {
    #[serde(default)]
    tokens: BTreeMap<String, CredentialToken>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialToken {
    url: String,
    credential: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct Consent {
    project: PathBuf,
    token: String,
    prefix: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConsentStore {
    #[serde(default)]
    grants: BTreeSet<Consent>,
}

struct CredentialMount {
    name: String,
    source: PathBuf,
}

struct HostCredentials {
    project: PathBuf,
    tokens: BTreeMap<String, CredentialToken>,
    consent_path: PathBuf,
    consent: ConsentStore,
    allow_secret: bool,
}

impl HostCredentials {
    fn load(project: &Path, allow_secret: bool) -> Result<Self> {
        let project = project
            .canonicalize()
            .context("canonicalizing FETCH credential project")?;
        let config_path = credential_config_path()?;
        let tokens = match fs::read(&config_path) {
            Ok(bytes) => {
                serde_json::from_slice::<CredentialsFile>(&bytes)
                    .with_context(|| {
                        format!("parsing FETCH credentials file {}", config_path.display())
                    })?
                    .tokens
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error).context("reading FETCH credentials file"),
        };
        let consent_path = consent_store_path()?;
        let consent = match fs::read(&consent_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("parsing FETCH consent store")?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => ConsentStore::default(),
            Err(error) => return Err(error).context("reading FETCH consent store"),
        };
        Ok(Self {
            project,
            tokens,
            consent_path,
            consent,
            allow_secret,
        })
    }

    fn for_command(&mut self, command: &str) -> Result<Option<CredentialMount>> {
        let Some(url) = concrete_fetch_url(command) else {
            return Ok(None);
        };
        let prefix = url_prefix(&url)?;
        let matches = self
            .tokens
            .iter()
            .filter(|(_, token)| token_matches(&token.url, &url))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let existing = self
            .consent
            .grants
            .iter()
            .find(|grant| grant.project == self.project && grant.prefix == prefix)
            .cloned();
        if matches.is_empty() {
            if let Some(grant) = existing {
                bail!("FETCH of {url} needs previously approved token {}; that token is no longer configured (refusing anonymous retry)", grant.token);
            }
            return Ok(None);
        }
        let name = if let Some(grant) = existing {
            if !matches.contains(&grant.token) {
                bail!("FETCH of {url} needs previously approved token {}; that token is no longer configured for this URL (refusing anonymous retry)", grant.token);
            }
            grant.token
        } else if matches.len() == 1 {
            matches[0].clone()
        } else {
            choose_token(&url, &matches, self.allow_secret)?
        };
        let grant = Consent {
            project: self.project.clone(),
            token: name.clone(),
            prefix,
        };
        if !self.consent.grants.contains(&grant) && !self.allow_secret {
            eprint!("allow FETCH of {url} using {name}? y/N ");
            io::stderr().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
                bail!("FETCH credential use was not approved");
            }
            self.consent.grants.insert(grant);
            self.save()?;
        }
        let token = &self.tokens[&name];
        if !token.credential.is_file() {
            bail!("FETCH token {name} has no readable credential file (refusing anonymous retry)");
        }
        Ok(Some(CredentialMount {
            name,
            source: token.credential.clone(),
        }))
    }

    fn save(&self) -> Result<()> {
        let parent = self
            .consent_path
            .parent()
            .expect("consent state path has a parent");
        fs::create_dir_all(parent).context("creating FETCH consent state directory")?;
        let temporary =
            tempfile::NamedTempFile::new_in(parent).context("creating FETCH consent state")?;
        serde_json::to_writer_pretty(temporary.reopen()?, &self.consent)?;
        temporary
            .persist(&self.consent_path)
            .map_err(|error| error.error)
            .context("saving FETCH consent state")?;
        Ok(())
    }
}

pub fn revoke_fetch_consent(token: &str) -> Result<usize> {
    let path = consent_store_path()?;
    let mut store: ConsentStore = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("parsing FETCH consent store")?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error).context("reading FETCH consent store"),
    };
    let removed = revoke_from_store(&mut store, token);
    if removed != 0 {
        let parent = path.parent().expect("consent state path has a parent");
        let temporary =
            tempfile::NamedTempFile::new_in(parent).context("creating FETCH consent state")?;
        serde_json::to_writer_pretty(temporary.reopen()?, &store)?;
        temporary
            .persist(path)
            .map_err(|error| error.error)
            .context("saving FETCH consent state")?;
    }
    Ok(removed)
}

fn revoke_from_store(store: &mut ConsentStore, token: &str) -> usize {
    let before = store.grants.len();
    store.grants.retain(|grant| grant.token != token);
    before - store.grants.len()
}

fn credential_config_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("CREDENTIALS_DIRECTORY") {
        return Ok(PathBuf::from(directory).join("credentials"));
    }
    let home = std::env::var_os("HOME")
        .context("HOME is unset; set CREDENTIALS_DIRECTORY for FETCH credentials")?;
    Ok(PathBuf::from(home).join(".config/cix/credentials"))
}

fn consent_store_path() -> Result<PathBuf> {
    if let Some(directory) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(directory).join("cix/fetch-consents.json"));
    }
    let home = std::env::var_os("HOME")
        .context("HOME is unset; set XDG_STATE_HOME for FETCH consent state")?;
    Ok(PathBuf::from(home).join(".local/state/cix/fetch-consents.json"))
}

fn concrete_fetch_url(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '\"']))
        .find(|word| word.starts_with("https://") || word.starts_with("http://"))
        .map(ToOwned::to_owned)
}

fn url_prefix(url: &str) -> Result<String> {
    let (_, after_scheme) = url
        .split_once("://")
        .context("FETCH URL must have a scheme")?;
    let (host, path) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    let first = path.split('/').next().filter(|part| !part.is_empty());
    Ok(match first {
        Some(first) => format!(
            "{}://{host}/{first}",
            &url[..url.find("://").expect("scheme exists")]
        ),
        None => format!(
            "{}://{host}",
            &url[..url.find("://").expect("scheme exists")]
        ),
    })
}

fn token_matches(pattern: &str, url: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => url.starts_with(prefix) && url.ends_with(suffix),
        None => url.starts_with(pattern),
    }
}

fn choose_token(url: &str, matches: &[String], allow_secret: bool) -> Result<String> {
    if allow_secret {
        bail!(
            "FETCH of {url} matches multiple credentials ({}); --allow-secret cannot choose one",
            matches.join(", ")
        );
    }
    eprint!(
        "FETCH of {url} matches credentials {}; choose token name: ",
        matches.join(", ")
    );
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    if matches.iter().any(|name| name == answer) {
        Ok(answer.to_owned())
    } else {
        bail!("no matching FETCH credential was selected")
    }
}
*/

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
) -> Result<(String, bool)> {
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
    // Store paths are complete by store invariant (the ensure_store_path
    // assumption); realization is only needed when an offer is missing.
    if context.offers.iter().any(|path| !Path::new(path).exists()) {
        realize_fetch_offers(cixfile, name, directory, lock, system, binders)?;
    }
    let offered_closure = query_closure(&context.offers)?;
    let shell = find_shell(&context.imports)?;
    let environment = build_environment(context.environment.clone());
    let command = &context.commands[0];
    let universe_identities = context
        .universe_identities
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let trace_key = step_memo_key(StepMemoKeyRequest {
        builder: name,
        index: 0,
        kind: "FETCH",
        directive: &fetch.source,
        arguments: command,
        offered_closure: &offered_closure,
        ordered_imports: &context.imports,
        environment: &environment,
        universe_identities: &universe_identities,
    })?;
    let trace_owner = format!("fetch:{name}");
    if needed.is_empty() {
        needed.insert(".".into(), NeededPath::default());
    }
    let existing_pin = lock.fetches.get(name).map(FetchPin::key);
    let existing_key = existing_pin
        .map(|pin| {
            top_fetch_chain_key(
                command,
                &offered_closure,
                &environment,
                &pin,
                &context
                    .universe_identities
                    .values()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        })
        .transpose()?;
    if cold && !force {
        if let Some(memo) = lock
            .step_memo
            .get(&trace_owner)
            .filter(|memo| memo.key == trace_key)
        {
            let empty = tempfile::tempdir().context("creating cold top-level FETCH audit root")?;
            verify_cold_read_set(memo, empty.path(), fetch.line, &fetch.source)?;
        }
        let pin = lock.fetches.get(name).with_context(|| {
            format!("FETCH {name} has no pin to replay; --cold never refetches")
        })?;
        let snapshot = replay_fetch_snapshot(directory, name, pin)?;
        verify_fetch_hash(fetch.expected.as_deref(), Some(pin), None)?;
        let paths = store_consumed_paths(Path::new(&snapshot), &needed)?;
        let key = top_fetch_chain_key(
            command,
            &offered_closure,
            &environment,
            &pin.key(),
            &context
                .universe_identities
                .values()
                .cloned()
                .collect::<Vec<_>>(),
        )?;
        lock.memo.insert(key.clone(), memo_entry(paths.clone()));
        let view = materialize_view(&paths)?;
        eprintln!(
            "FETCH {name} replayed pinned snapshot {} -> {view}",
            short_key(&key)
        );
        return Ok((view, false));
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
                return Ok((view, false));
            }
        }
    }
    let work = tempfile::Builder::new()
        .prefix("cix-fetch-work-")
        .tempdir()
        .context("creating top-level FETCH workdir")?;
    let trace_before = copied_snapshot(work.path())?;
    let started = Instant::now();
    let credential = credentials.for_command(command)?;
    let observations = run_sandbox(
        work.path(),
        &shell,
        command,
        &environment,
        &BTreeMap::new(),
        &offered_closure,
        &context.imports,
        None,
        credential
            .as_ref()
            .into_iter()
            .collect::<Vec<_>>()
            .as_slice(),
    )
    .with_context(|| {
        format!(
            "line {}: top-level FETCH {name:?} failed\n  | {:?}",
            fetch.line, fetch.source
        )
    })?;
    let mut step_volatile = BTreeSet::new();
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
            credential
                .as_ref()
                .into_iter()
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .with_context(|| {
            format!(
                "line {}: top-level FETCH {name:?} probe failed\n  | {:?}",
                fetch.line, fetch.source
            )
        })?;
        let observed_volatile = volatile_paths(first.path(), work.path())?;
        report_volatile(name, &observed_volatile);
        step_volatile.extend(observed_volatile.keys().cloned());
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
    let mut reads = trace::read_dependencies(trace_before.path(), &observations)?;
    let mut changes =
        trace::filesystem_changes(trace_before.path(), work.path(), &observations.writes)?;
    retain_nonvolatile_reads(&mut reads, &step_volatile);
    trace::record_workspace_fingerprints(work.path(), &mut reads, &observations.writes)?;
    changes.retain(|path, _| !path_overlaps_any(path, &step_volatile));
    let output_hashes = memo_output_hashes(work.path(), &changes)?;
    let step_output = (!changes.is_empty())
        .then(|| add_step_output_snapshot(work.path(), &changes, &step_volatile))
        .transpose()?;
    lock.step_memo.insert(
        trace_owner,
        StepMemo {
            key: trace_key,
            reads,
            output_snapshot: step_output,
            changes,
            output_hashes,
        },
    );
    trace_before.close()?;
    let actual_paths = fetch_path_hashes(work.path(), &needed)?;
    report_unconsumed_complement(name, work.path(), &needed);
    let pin = lock.fetches.get(name).cloned();
    let refreshed = refresh_fetch_pin(
        pin.as_ref(),
        fetch.expected.is_some(),
        force,
        actual_paths,
        &output_hash,
        volatile,
        name,
    )?;
    cache_fetch_snapshot(directory, name, &refreshed, &snapshot)?;
    lock.fetches.insert(name.to_owned(), refreshed);
    let pin = lock.fetches[name].key();
    let key = top_fetch_chain_key(
        command,
        &offered_closure,
        &environment,
        &pin,
        &context
            .universe_identities
            .values()
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    let paths = store_consumed_paths(work.path(), &needed)?;
    lock.memo.insert(key.clone(), memo_entry(paths.clone()));
    let view = materialize_view(&paths)?;
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
) -> Result<(String, Vec<ExecutedStep>)> {
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
        // Store paths are complete by store invariant (the ensure_store_path
        // assumption); realization is only needed when an offer is missing.
        if context.offers.iter().any(|path| !Path::new(path).exists()) {
            realize_builder_offers(cixfile, builder_name, directory, lock, system, binders)?;
        }
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
    let mut environment = vendored_dev_environment(
        cixfile,
        builder_name,
        directory,
        lock,
        system,
        binders,
        &context.imports,
        &context.universe_identities,
    )?;
    environment.extend(context.environment.clone());
    environment = build_environment(environment);
    let universe_identities = context
        .universe_identities
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut export_prelude = BTreeMap::new();
    install_declared_expectations(builder_name, builder, &context.commands, lock);
    let chain_key_started = Instant::now();
    let existing_keys = builder_chain_keys(
        builder_name,
        builder,
        &context,
        &offered_closure,
        &environment,
        lock,
    )?;
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
            if memo_has_paths(lock.memo.get(key), &needed)? {
                let view = materialize_view(&lock.memo[key].paths)?;
                eprintln!(
                    "BUILDER {builder_name} memo hit {} -> {view}",
                    short_key(key)
                );
                return Ok((view, builder_step_results(builder_name, builder, false)));
            }
        }
    }

    let persistent = (!cold)
        .then(|| workspace_paths(workspace_directory, directory, builder_name))
        .transpose()?;
    let prior_state = persistent
        .as_ref()
        .and_then(|paths| load_workspace_state(&paths.2))
        .unwrap_or_default();
    let prior_keys = prior_state.step_keys;
    let mut materialized_memos = prior_state.materialized_memos;
    let mut output_fingerprints_by_memo = prior_state.memo_output_fingerprints;
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
    let mut step_results = Vec::with_capacity(builder.steps.len());
    let mut fetch_snapshots =
        BTreeMap::<String, (bool, Option<String>, String, BTreeMap<String, VolatilePath>)>::new();
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
                step_results.push(executed_step(builder_name, index, "ENV", true));
            }
            BuildStep::Copy(copy) => {
                let resolved_source = &context.copies[copy_index];
                copy_index += 1;
                let staging_started = Instant::now();
                stage_input(
                    Path::new(resolved_source),
                    &copy.dst,
                    &workdir,
                    &staging.join(format!("step-{index}")),
                )
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
                step_results.push(executed_step(builder_name, index, "COPY", true));
            }
            BuildStep::Fetch { line, source, .. } | BuildStep::Run { line, source, .. } => {
                let command = &context.commands[command_index];
                command_index += 1;
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
                    step_results.push(executed_step(builder_name, index, kind, false));
                    continue;
                }
                let is_fetch = matches!(step, BuildStep::Fetch { .. });
                let kind = if is_fetch { "FETCH" } else { "RUN" };
                let memo_key = step_memo_key(StepMemoKeyRequest {
                    builder: builder_name,
                    index,
                    kind,
                    directive: source,
                    arguments: command,
                    offered_closure: &offered_closure,
                    ordered_imports: &context.imports,
                    environment: &environment,
                    universe_identities: &universe_identities,
                })?;
                let memo_owner = format!("builder:{builder_name}:{index}");
                let superseded_memo = lock.step_memo.get(&memo_owner).cloned();
                let recorded_memo = superseded_memo
                    .as_ref()
                    .filter(|memo| memo.key == memo_key)
                    .cloned();
                let fetch_id = is_fetch
                    .then(|| format!("builder:{builder_name}:{}", fetch_id(index, command)));
                if let Some(id) = &fetch_id {
                    if cold {
                        if let Some(memo) = &recorded_memo {
                            verify_cold_read_set(memo, &workdir, *line, source)?;
                        }
                        let pin = lock.fetches.get(id).with_context(|| {
                            format!(
                                "BUILDER {builder_name} FETCH has no pin to replay; --cold never refetches"
                            )
                        })?;
                        if let Some(memo) = &recorded_memo {
                            apply_step_memo(memo, &workdir, None)?;
                        } else {
                            let snapshot = replay_fetch_snapshot(directory, id, pin)?;
                            restore_snapshot(Path::new(&snapshot), &workdir)?;
                        }
                        eprintln!(
                            "BUILDER {builder_name} step {} FETCH replayed pinned snapshot",
                            index + 1
                        );
                        step_results.push(executed_step(builder_name, index, kind, false));
                        continue;
                    }
                }
                let mut known_reads = None;
                if !cold && !update_fetch_pins {
                    if let Some(memo) = recorded_memo.clone() {
                        let fingerprints = is_fetch
                            .then(|| output_fingerprints_by_memo.get(&memo_owner))
                            .flatten()
                            .filter(|_| materialized_memos.get(&memo_owner) == Some(&memo_key));
                        let (matches, current) =
                            validate_step_memo(&memo, &workdir, is_fetch, fingerprints)?;
                        known_reads = Some(current);
                        if !newly_consumed_paths && matches {
                            if is_fetch {
                                // A FETCH hit is constructive: its full recorded write set is
                                // re-applied even when this workspace previously held it. That
                                // keeps the self-read rule below tied to one complete output.
                                apply_step_memo(&memo, &workdir, fingerprints)?;
                                materialized_memos.insert(memo_owner.clone(), memo_key.clone());
                                output_fingerprints_by_memo.insert(
                                    memo_owner.clone(),
                                    memo_output_fingerprints(&workdir, &memo.changes)?,
                                );
                            } else if materialized_memos.get(&memo_owner) == Some(&memo_key) {
                                crate::cix_timing!(
                                    "CIX timing memo-apply skipped=workspace-already-materialized"
                                );
                            } else {
                                apply_step_memo(&memo, &workdir, None)?;
                                materialized_memos.insert(memo_owner.clone(), memo_key.clone());
                            }
                            eprintln!(
                                "BUILDER {builder_name} step {} {kind} memo hit {}",
                                index + 1,
                                short_key(&memo_key)
                            );
                            step_results.push(executed_step(builder_name, index, kind, false));
                            continue;
                        }
                    }
                }
                if is_fetch && !cold {
                    if let Some(memo) = &superseded_memo {
                        crate::cix_timing!(
                            "CIX timing fetch-revert owner={} key={}",
                            memo_owner,
                            short_key(&memo.key)
                        );
                        revert_step_writes(memo, &workdir)?;
                    }
                }
                let snapshot_started = Instant::now();
                let trace_before = copied_snapshot(&workdir)?;
                crate::cix_timing!(
                    "CIX timing workspace-snapshot phase=before-command wall_ms={}",
                    snapshot_started.elapsed().as_millis()
                );
                let probe_before = (is_fetch
                    && update_fetch_pins
                    && matches!(step, BuildStep::Fetch { expected: None, .. }))
                .then(|| copied_snapshot(&workdir))
                .transpose()?;
                let started = Instant::now();
                let credential = if is_fetch {
                    credentials.for_command(command)?
                } else {
                    None
                };
                let observations = run_sandbox(
                    &workdir,
                    shell.as_deref().expect("command steps have a shell"),
                    command,
                    &environment,
                    &export_prelude,
                    &offered_closure,
                    &context.imports,
                    if is_fetch { None } else { run_network },
                    credential
                        .as_ref()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
                .with_context(|| format!("line {line}: {kind} failed\n  | {source:?}"))?;
                let read_set_started = Instant::now();
                let empty_reads = BTreeMap::new();
                let (mut reads, recording_metrics) = trace::read_dependencies_with_known(
                    trace_before.path(),
                    &observations,
                    known_reads.as_ref().unwrap_or(&empty_reads),
                )?;
                crate::cix_timing!(
                    "CIX timing trace-read-set reused={} hashed_files={} hashed_bytes={} hashed_directories={} wall_ms={}",
                    recording_metrics.reused,
                    recording_metrics.hashed_files,
                    recording_metrics.hashed_bytes,
                    recording_metrics.hashed_directories,
                    read_set_started.elapsed().as_millis()
                );
                let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let mut step_volatile = BTreeSet::new();
                if is_fetch {
                    let id = fetch_id.expect("FETCH has an id");
                    let volatile = if let Some(before) = probe_before {
                        let first = copied_snapshot(&workdir)?;
                        replace_workspace_tree(before.path(), &workdir)?;
                        let _ = run_sandbox(
                            &workdir,
                            shell.as_deref().expect("command steps have a shell"),
                            command,
                            &environment,
                            &export_prelude,
                            &offered_closure,
                            &context.imports,
                            None,
                            credential
                                .as_ref()
                                .into_iter()
                                .collect::<Vec<_>>()
                                .as_slice(),
                        )
                        .with_context(|| {
                            format!("line {line}: FETCH update probe failed\n  | {source:?}")
                        })?;
                        let observed_volatile = volatile_paths(first.path(), &workdir)?;
                        report_volatile(&id, &observed_volatile);
                        step_volatile.extend(observed_volatile.keys().cloned());
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
                    fetch_snapshots.insert(
                        id,
                        (expected.is_some(), Some(snapshot.clone()), actual, volatile),
                    );
                }
                let changes_started = Instant::now();
                let mut changes =
                    trace::filesystem_changes(trace_before.path(), &workdir, &observations.writes)?;
                crate::cix_timing!(
                    "CIX timing workspace-delta wall_ms={}",
                    changes_started.elapsed().as_millis()
                );
                if let Some(previous) = &recorded_memo {
                    retain_replay_roots(previous, &workdir, &mut changes)?;
                }
                if is_fetch {
                    retain_fetch_output_roots(trace_before.path(), &workdir, &mut changes)?;
                }
                retain_nonvolatile_reads(&mut reads, &step_volatile);
                trace::record_workspace_fingerprints(&workdir, &mut reads, &observations.writes)?;
                changes.retain(|path, _| !path_overlaps_any(path, &step_volatile));
                if !is_fetch {
                    invalidate_fetch_output_fingerprints(
                        &mut output_fingerprints_by_memo,
                        &materialized_memos,
                        lock,
                        &changes,
                    );
                }
                let output_hashes = is_fetch
                    .then(|| memo_output_hashes(&workdir, &changes))
                    .transpose()?
                    .unwrap_or_default();
                let output_fingerprints = is_fetch
                    .then(|| memo_output_fingerprints(&workdir, &changes))
                    .transpose()?
                    .unwrap_or_default();
                if cold {
                    if let Some(recorded) = &recorded_memo {
                        compare_cold_read_sets(recorded, &reads, *line, source)?;
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
                                add_step_output_snapshot(&workdir, &changes, &step_volatile);
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
                    materialized_memos.insert(memo_owner.clone(), memo_key.clone());
                    if is_fetch {
                        output_fingerprints_by_memo.insert(memo_owner.clone(), output_fingerprints);
                    }
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
                step_results.push(executed_step(builder_name, index, kind, true));
            }
        }
    }
    if !fetch_snapshots.is_empty() {
        let actual_paths = fetch_path_hashes(&workdir, &needed)?;
        report_unconsumed_complement(builder_name, &workdir, &needed);
        for (id, (expected, snapshot, snapshot_nar_hash, volatile)) in fetch_snapshots {
            let refreshed = refresh_fetch_pin(
                lock.fetches.get(&id),
                expected,
                update_fetch_pins,
                actual_paths.clone(),
                &snapshot_nar_hash,
                volatile,
                &id,
            )?;
            if let Some(snapshot) = snapshot {
                cache_fetch_snapshot(directory, &id, &refreshed, &snapshot)?;
            }
            lock.fetches.insert(id, refreshed);
        }
    }
    let chain_key_started = Instant::now();
    let step_keys = builder_chain_keys(
        builder_name,
        builder,
        &context,
        &offered_closure,
        &environment,
        lock,
    )?
    .context("builder chain still has an unpinned FETCH after execution")?;
    crate::cix_timing!(
        "CIX timing chain-keys phase=final wall_ms={}",
        chain_key_started.elapsed().as_millis()
    );
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
        refresh_fetch_output_fingerprints(
            &mut output_fingerprints_by_memo,
            &materialized_memos,
            lock,
            &workdir,
        )?;
        save_workspace_state(
            &persistent.2,
            &WorkspaceState {
                step_keys: step_keys.clone(),
                materialized_memos,
                memo_output_fingerprints: output_fingerprints_by_memo,
            },
        )?;
    }
    let view = materialize_view(&paths)?;
    eprintln!(
        "BUILDER {builder_name} memo miss {} -> {view}",
        short_key(&key)
    );
    Ok((view, step_results))
}

fn invalidate_fetch_output_fingerprints(
    fingerprints: &mut BTreeMap<String, BTreeMap<String, String>>,
    materialized: &BTreeMap<String, String>,
    lock: &LockFile,
    changes: &BTreeMap<String, StepChange>,
) {
    fingerprints.retain(|owner, _| {
        let Some(memo) = lock.step_memo.get(owner) else {
            return false;
        };
        materialized.get(owner) == Some(&memo.key)
            && !memo.changes.keys().any(|output| {
                changes.keys().any(|changed| {
                    same_or_descendant(output, changed) || same_or_descendant(changed, output)
                })
            })
    });
}

fn refresh_fetch_output_fingerprints(
    fingerprints: &mut BTreeMap<String, BTreeMap<String, String>>,
    materialized: &BTreeMap<String, String>,
    lock: &LockFile,
    workspace: &Path,
) -> Result<()> {
    for (owner, output_fingerprints) in fingerprints {
        let Some(memo) = lock.step_memo.get(owner) else {
            continue;
        };
        if materialized.get(owner) == Some(&memo.key) {
            *output_fingerprints = memo_output_fingerprints(workspace, &memo.changes)?;
        }
    }
    Ok(())
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

#[allow(clippy::too_many_arguments)]
fn vendored_dev_environment(
    cixfile: &Cixfile,
    builder_name: &str,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    snapshots: &BTreeMap<String, String>,
    imports: &[String],
    universe_identities: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    if imports.is_empty() {
        return Ok(BTreeMap::new());
    }
    let universe = cixfile
        .inputs
        .iter()
        .find(|(_, input)| input.kind == crate::InputKind::PackageUniverse)
        .map(|(name, _)| name)
        .context("BUILDER IMPORT needs a package-universe FROM")?;
    let identity = universe_identities
        .get(universe)
        .context("package universe identity was not resolved")?;
    let key = format!("{identity}:{}", hex_hash(imports.join("\0").as_bytes()));
    if let Some(snapshot) = lock.dev_envs.get(&key) {
        lock.builder_dev_envs
            .insert(builder_name.to_owned(), key.clone());
        let environment = filter_development_environment(&snapshot.environment);
        if environment != snapshot.environment {
            lock.dev_envs.insert(
                key,
                DevEnvironment {
                    environment: environment.clone(),
                },
            );
        }
        return Ok(environment);
    }
    let expression =
        generate_builder_dev_env_nix(cixfile, builder_name, directory, lock, system, snapshots)?;
    let raw = cix_common::nix(&["print-dev-env", "--impure", "--json", "--expr", &expression])
        .context("capturing nixpkgs development environment for IMPORT")?;
    let document: serde_json::Value =
        serde_json::from_str(&raw).context("parsing nix print-dev-env JSON")?;
    let raw_environment = document
        .get("variables")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|variables| variables.iter())
        .filter_map(|(name, variable)| {
            variable
                .get("value")?
                .as_str()
                .map(|value| (name.clone(), value.to_owned()))
        })
        .collect::<BTreeMap<_, _>>();
    let environment = filter_development_environment(&raw_environment);
    lock.dev_envs.insert(
        key.clone(),
        DevEnvironment {
            environment: environment.clone(),
        },
    );
    lock.builder_dev_envs.insert(builder_name.to_owned(), key);
    Ok(environment)
}

fn filter_development_environment(
    environment: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    environment
        .iter()
        .filter(|(name, value)| {
            value.contains("/nix/store/")
                && !value.contains(char::is_whitespace)
                && !skeleton_environment_variable(name)
                && development_search_variable(name)
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn skeleton_environment_variable(name: &str) -> bool {
    matches!(
        name,
        "BASH"
            | "CONFIG_SHELL"
            | "HOME"
            | "HOST_PATH"
            | "LC_ALL"
            | "PATH"
            | "SHELL"
            | "SOURCE_DATE_EPOCH"
            | "SSL_CERT_FILE"
            | "TMPDIR"
            | "TZ"
    ) || name.starts_with("NIX_")
        || name.starts_with("stdenv")
        || name.ends_with("Phase")
        || name.ends_with("Hooks")
}

fn development_search_variable(name: &str) -> bool {
    name.ends_with("_PATH")
        || name.ends_with("_DIRS")
        || matches!(
            name,
            "PKG_CONFIG_PATH" | "CMAKE_PREFIX_PATH" | "SYSTEM_CERTIFICATE_PATH"
        )
}

fn report_unconsumed_complement(
    name: &str,
    workspace: &Path,
    needed: &BTreeMap<String, NeededPath>,
) {
    const THRESHOLD_BYTES: u64 = 16 * 1024 * 1024;
    let total = tree_size(workspace).unwrap_or(0);
    let consumed = needed
        .keys()
        .map(|path| {
            let source = if path == "." {
                workspace.to_owned()
            } else {
                workspace.join(path)
            };
            tree_size(&source).unwrap_or(0)
        })
        .sum::<u64>();
    let complement = total.saturating_sub(consumed.min(total));
    if complement >= THRESHOLD_BYTES {
        eprintln!(
            "note: FETCH {name} leaves {} MiB unconsumed of {} MiB in its workspace; only COPY-reachable paths enter the pin",
            complement / (1024 * 1024),
            total / (1024 * 1024),
        );
    }
}

fn tree_size(path: &Path) -> Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Ok(metadata.len());
    }
    fs::read_dir(path)?.try_fold(0u64, |total, entry| {
        let entry = entry?;
        Ok(total.saturating_add(tree_size(&entry.path())?))
    })
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
    let universe_identities = context
        .universe_identities
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for (index, step) in builder.steps.iter().enumerate() {
        let (kind, arguments, sources, fetch_pin) = match step {
            BuildStep::Env { name, value, .. } => {
                let value = value
                    .literal_value()
                    .context("builder ENV metadata was not resolved")?;
                environment.insert(name.clone(), value.clone());
                ("ENV", format!("{name}={value}"), Vec::new(), None)
            }
            BuildStep::Copy(copy) => {
                let source = &context.copies[copy_index];
                copy_index += 1;
                (
                    "COPY",
                    copy_key_arguments(copy)?,
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
            universe_identities: &universe_identities,
        })?;
        keys.push(predecessor.clone());
    }
    Ok(Some(keys))
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
            } => {
                unreachable!("unresolved FROM metadata {namespace}.{attribute}")
            }
        })
        .collect();
    Ok(serde_json::to_string(&CopyKey {
        src,
        dst: &copy.dst,
    })?)
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

fn validate_step_memo(
    memo: &StepMemo,
    workspace: &Path,
    allow_fetch_self_reads: bool,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<(bool, BTreeMap<String, crate::ReadDependency>)> {
    let validation_started = Instant::now();
    let replayable = if !memo.changes.is_empty() {
        let Some(snapshot) = memo.output_snapshot.as_deref() else {
            return Ok((false, trace::current_dependencies(workspace, &memo.reads)?));
        };
        ensure_store_path(snapshot)?
    } else {
        true
    };
    let (current, metrics) = trace::current_dependencies_with_metrics(workspace, &memo.reads)?;
    crate::cix_timing!(
        "CIX timing memo-validation rehashed_files={} rehashed_bytes={}",
        metrics.rehashed_files,
        metrics.rehashed_bytes
    );
    let self_matches = if current == memo.reads || !allow_fetch_self_reads || !replayable {
        false
    } else {
        let self_validation_started = Instant::now();
        let matches = memo_write_set_matches_workspace(memo, workspace, output_fingerprints)?
            && memo_self_reads_match(memo, workspace, &current, output_fingerprints)?;
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

/// A FETCH may observe output from its own prior execution only when every
/// recorded output path still equals that memo's constructive snapshot.
fn memo_write_set_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    for (path, change) in &memo.changes {
        let workspace_path = workspace.join(path);
        match change {
            StepChange::Absent => match fs::symlink_metadata(&workspace_path) {
                Ok(_) => return Ok(false),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            },
            StepChange::Present | StepChange::Directory { .. } => {
                if !memo_output_matches_workspace(memo, workspace, path, output_fingerprints)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn memo_self_reads_match(
    memo: &StepMemo,
    workspace: &Path,
    current: &BTreeMap<String, crate::ReadDependency>,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    for (path, recorded) in &memo.reads {
        if current.get(path) != Some(recorded)
            && !memo_path_matches_workspace(memo, workspace, path, output_fingerprints)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn memo_path_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    path: &str,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    let Some(snapshot) = memo.output_snapshot.as_deref() else {
        return Ok(false);
    };
    if !memo
        .changes
        .keys()
        .any(|written| same_or_descendant(path, written) || same_or_descendant(written, path))
    {
        return Ok(false);
    }
    if let Some((root, _)) = memo
        .output_hashes
        .iter()
        .find(|(root, _)| same_or_descendant(path, root))
    {
        return memo_output_matches_workspace(memo, workspace, root, output_fingerprints);
    }
    Ok(node_content_hash(&Path::new(snapshot).join(path))?
        == node_content_hash(&workspace.join(path))?)
}

fn memo_output_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    path: &str,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    let Some(expected) = memo.output_hashes.get(path) else {
        return Ok(false);
    };
    let current = workspace.join(path);
    let fingerprint = node_fingerprint(&current)?;
    if fingerprint.as_ref().is_some_and(|fingerprint| {
        output_fingerprints.and_then(|fingerprints| fingerprints.get(path)) == Some(fingerprint)
    }) {
        return Ok(true);
    }
    crate::cix_timing!(
        "CIX timing memo-output-fingerprint-miss path={} actual={}",
        path,
        fingerprint.as_deref().unwrap_or("<absent>")
    );
    Ok(node_content_hash(&current)? == Some(expected.content.clone()))
}

fn memo_output_hashes(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
) -> Result<BTreeMap<String, crate::OutputHash>> {
    changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| {
            let output = workspace.join(path);
            Ok((
                path.clone(),
                crate::OutputHash {
                    content: node_content_hash(&output)?.context("memo output disappeared")?,
                },
            ))
        })
        .collect()
}

fn memo_output_fingerprints(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
) -> Result<BTreeMap<String, String>> {
    changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| {
            Ok((
                path.clone(),
                node_fingerprint(&workspace.join(path))?.context("memo output disappeared")?,
            ))
        })
        .collect()
}

fn node_content_hash(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut digest = Sha256::new();
    // Store snapshots are read-only, while the warm workspace must remain
    // writable; executable bits are the output's only mode-level content.
    digest.update((metadata.permissions().mode() & 0o111).to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if metadata.is_file() {
        digest.update(b"file\0");
        let mut file = fs::File::open(path)?;
        io::copy(&mut file, &mut digest)?;
    } else if metadata.is_dir() {
        digest.update(b"directory\0");
        let mut entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            digest.update(entry.file_name().as_encoded_bytes());
            digest.update([0]);
            digest.update(
                node_content_hash(&entry.path())?
                    .unwrap_or_default()
                    .as_bytes(),
            );
            digest.update([0]);
        }
    } else {
        bail!(
            "unsupported special file in memo output: {}",
            path.display()
        );
    }
    Ok(Some(hex_hash(&digest.finalize())))
}

fn node_fingerprint(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut digest = Sha256::new();
    // This is only the same nonsemantic metadata fast path used for traced
    // reads. A miss below always rechecks the persisted content hash.
    digest.update(metadata.dev().to_le_bytes());
    digest.update(metadata.permissions().mode().to_le_bytes());
    digest.update(metadata.len().to_le_bytes());
    digest.update(metadata.ino().to_le_bytes());
    digest.update(metadata.mtime_nsec().to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if metadata.is_file() {
        digest.update(b"file\0");
    } else if metadata.is_dir() {
        digest.update(b"directory\0");
    } else {
        bail!(
            "unsupported special file in memo output: {}",
            path.display()
        );
    }
    Ok(Some(hex_hash(&digest.finalize())))
}

fn revert_step_writes(memo: &StepMemo, workspace: &Path) -> Result<()> {
    let mut paths = memo
        .changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| path.matches('/').count());
    let mut reverted = Vec::<&str>::new();
    for path in paths {
        if reverted
            .iter()
            .any(|parent| same_or_descendant(path, parent))
        {
            continue;
        }
        remove_path_if_present(&workspace.join(path))?;
        reverted.push(path);
    }
    Ok(())
}

fn apply_step_memo(
    memo: &StepMemo,
    workspace: &Path,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<()> {
    if memo.changes.is_empty() {
        return Ok(());
    }
    let snapshot = Path::new(
        memo.output_snapshot
            .as_deref()
            .context("step memo with filesystem changes has no output snapshot")?,
    );
    let mut absent = memo
        .changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Absent))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    absent.sort_by_key(|path| std::cmp::Reverse(path.matches('/').count()));
    for relative in absent {
        remove_path_if_present(&workspace.join(relative))?;
    }
    let mut present = memo
        .changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Present))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    present.sort_by_key(|path| path.matches('/').count());
    let apply_started = Instant::now();
    let mut synced = Vec::<&str>::new();
    for relative in present {
        if synced.iter().any(|parent| {
            relative
                .strip_prefix(*parent)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            continue;
        }
        if memo_output_matches_workspace(memo, workspace, relative, output_fingerprints)? {
            continue;
        }
        sync_replay_node(&snapshot.join(relative), &workspace.join(relative))?;
        synced.push(relative);
    }
    crate::cix_timing!(
        "CIX timing memo-apply roots={} wall_ms={}",
        synced.len(),
        apply_started.elapsed().as_millis()
    );
    for (relative, change) in &memo.changes {
        let StepChange::Directory { mode } = change else {
            continue;
        };
        let path = if relative == "." {
            workspace.to_owned()
        } else {
            workspace.join(relative)
        };
        fs::set_permissions(path, fs::Permissions::from_mode(*mode))?;
    }
    Ok(())
}

fn add_step_output_snapshot(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
    excluded: &BTreeSet<String>,
) -> Result<String> {
    let delta = tempfile::Builder::new()
        .prefix("cix-step-delta-")
        .tempdir()
        .context("creating step output delta")?;
    let mut present = changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Present))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    present.sort_by_key(|path| path.matches('/').count());
    let mut copied = Vec::<&str>::new();
    for relative in present {
        if copied.iter().any(|parent| {
            relative
                .strip_prefix(*parent)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            continue;
        }
        copy_node(&workspace.join(relative), &delta.path().join(relative))?;
        copied.push(relative);
    }
    for relative in excluded {
        remove_path_if_present(&delta.path().join(relative))?;
    }
    add_store_object(delta.path(), "cix-step-output")
}

fn retain_nonvolatile_reads(
    reads: &mut BTreeMap<String, crate::ReadDependency>,
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
            (StepChange::Present, Ok(_)) => {
                changes.retain(|candidate, _| !same_or_descendant(candidate, root));
                changes.insert(root.clone(), StepChange::Present);
            }
            (StepChange::Present | StepChange::Absent, Err(error))
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

/// A FETCH that creates a new top-level tree owns that complete tree, not just
/// the individual syscalls the tracer happened to observe beneath it. Keeping
/// that root makes constructive replay—and therefore self-observation—cover
/// every output file.
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

fn same_or_descendant(candidate: &str, root: &str) -> bool {
    candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
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
    cold: &BTreeMap<String, crate::ReadDependency>,
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

fn workspace_paths(
    base: &Path,
    directory: &Path,
    builder: &str,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
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
        // Unchanged staged input: leave the workspace node untouched so its
        // inode and mtime survive restaging (cargo-style mtime fingerprints
        // and the memo-validation fastpath both depend on this stability).
        (Some(old), Some(new), true) if nodes_equal(old, new)? => Ok(()),
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

/// Reconcile a workspace node with a memo output snapshot node, producing the
/// same end state as remove-and-recopy while leaving already-identical nodes
/// untouched so their inodes and mtimes stay stable across warm replays
/// (cargo-style mtime fingerprints and memo-validation fingerprints both
/// depend on that stability). Extra workspace entries under a replayed
/// directory are removed, exactly as the wholesale copy did.
fn sync_replay_node(source: &Path, destination: &Path) -> Result<()> {
    let source_meta = fs::symlink_metadata(source)?;
    let destination_meta = fs::symlink_metadata(destination).ok();
    if source_meta.is_dir() && !source_meta.file_type().is_symlink() {
        match &destination_meta {
            Some(existing) if existing.is_dir() && !existing.file_type().is_symlink() => {
                let mut names = BTreeSet::new();
                for entry in fs::read_dir(source)? {
                    names.insert(entry?.file_name());
                }
                for entry in fs::read_dir(destination)? {
                    let name = entry?.file_name();
                    if !names.contains(&name) {
                        remove_path_if_present(&destination.join(name))?;
                    }
                }
                for name in names {
                    sync_replay_node(&source.join(&name), &destination.join(&name))?;
                }
                Ok(())
            }
            _ => {
                remove_path_if_present(destination)?;
                copy_node(source, destination)?;
                make_writable(destination)
            }
        }
    } else if destination_meta.is_some() && nodes_equal(source, destination)? {
        Ok(())
    } else {
        remove_path_if_present(destination)?;
        copy_node(source, destination)?;
        make_writable(destination)
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
    if root_metadata.file_type().is_symlink() {
        return Ok(());
    }
    if !root_metadata.is_dir() {
        let mut permissions = root_metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o200);
        fs::set_permissions(path, permissions)?;
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
    cix_common::record_nix_subprocess();
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
    if let Some(context) = cached_context(&expression, directory)? {
        return Ok(context);
    }
    let context = eval_context(&expression)?;
    cache_context(&expression, directory, &context)?;
    Ok(context)
}

/// The generated context expression is byte-stable across source edits (the
/// source enters it only as a `builtins.path` literal of a fixed directory
/// path), so its evaluation result is reusable as long as every resolved
/// store path still exists — except `copies` entries rooted in the source,
/// which move with the source content. Those are re-rooted by store-adding
/// the source directory (`nix store add --mode nar` computes the identical
/// path to `builtins.path`). Expressions whose results depend on source
/// content beyond that root (hashFile interpolations, project-local
/// overlays) never take this fastpath.
fn context_source_dependent(expression: &str) -> bool {
    expression.contains("builtins.hashFile") || expression.contains("overlay = import")
}

fn context_cache_file(expression: &str) -> Result<Option<PathBuf>> {
    let base = if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        return Ok(None);
    };
    Ok(Some(
        base.join("cix/context-cache")
            .join(hex_hash(expression.as_bytes())),
    ))
}

#[derive(Serialize, Deserialize)]
struct CachedContext {
    source_root: Option<String>,
    context: BuildContext,
}

fn cached_context(expression: &str, directory: &Path) -> Result<Option<BuildContext>> {
    if context_source_dependent(expression) {
        return Ok(None);
    }
    let Some(file) = context_cache_file(expression)? else {
        return Ok(None);
    };
    let Some(cached) = fs::read(&file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CachedContext>(&bytes).ok())
    else {
        return Ok(None);
    };
    let mut context = cached.context;
    if let Some(old_root) = cached.source_root.as_deref() {
        let new_root = add_source_root(directory)?;
        for copy in &mut context.copies {
            if let Some(suffix) = copy.strip_prefix(old_root) {
                *copy = format!("{new_root}{suffix}");
            }
        }
    }
    let complete = context
        .offers
        .iter()
        .chain(&context.imports)
        .chain(&context.copies)
        .map(|path| trim_copy_suffix(path))
        .all(|path| Path::new(path).exists());
    if !complete {
        return Ok(None);
    }
    Ok(Some(context))
}

fn cache_context(expression: &str, directory: &Path, context: &BuildContext) -> Result<()> {
    if context_source_dependent(expression) {
        return Ok(());
    }
    let Some(file) = context_cache_file(expression)? else {
        return Ok(());
    };
    let source_root = if context
        .copies
        .iter()
        .any(|copy| copy.contains("-cix-source"))
    {
        Some(add_source_root(directory)?)
    } else {
        None
    };
    let parent = file.parent().expect("context cache file has a parent");
    fs::create_dir_all(parent)
        .with_context(|| format!("creating context cache {}", parent.display()))?;
    let payload = serde_json::to_vec(&CachedContext {
        source_root,
        context: context.clone(),
    })?;
    let temporary = file.with_extension("next");
    fs::write(&temporary, payload)
        .with_context(|| format!("writing context cache {}", temporary.display()))?;
    fs::rename(&temporary, &file)
        .with_context(|| format!("recording context cache {}", file.display()))?;
    Ok(())
}

fn add_source_root(directory: &Path) -> Result<String> {
    let directory = directory
        .canonicalize()
        .with_context(|| format!("resolving Cixfile source {}", directory.display()))?;
    let path = directory
        .to_str()
        .context("Cixfile source directory is not UTF-8")?;
    let added = cix_common::nix(&[
        "store",
        "add",
        "--mode",
        "nar",
        "--name",
        "cix-source",
        path,
    ])?;
    added
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .context("nix store add did not return a source store path")
}

fn trim_copy_suffix(path: &str) -> &str {
    path.strip_suffix('/').unwrap_or(path)
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
    cix_common::record_nix_subprocess();
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
    credentials: &[&CredentialMount],
) -> Result<trace::Capture> {
    let import_union = prepare_import_union(imports, run_network.is_none())?;
    let loader_surface = fhs::LoaderSurface::new(imports)?;
    let env_is_missing = !import_union.path().join("bin/env").is_file();
    let trace_directory = tempfile::Builder::new()
        .prefix("cix-read-trace-")
        .tempdir()
        .context("creating read trace directory")?;
    let trace_path = trace_directory.path().join("syscalls");
    let mut process = Command::new("strace");
    process
        .args(["-f", "--seccomp-bpf", "-qq", "-yy", "-s", "0", "-e"])
        .arg("trace=%file,getdents,getdents64,chdir,fchdir,clone,clone3,fork,vfork")
        .arg("-o")
        .arg(&trace_path)
        .args(["--", "bwrap"]);
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
    let seccomp_filter = if run_network == Some(RunNetwork::SocketFilter) {
        Some(seccomp::prepare_socket_filter(&mut process)?)
    } else {
        None
    };
    if let Some(filter) = &seccomp_filter {
        process.arg("--seccomp").arg(filter.as_raw_fd().to_string());
    }
    process.args(["--dir", "/nix", "--dir", "/nix/store"]);
    process.args(["--dir", "/usr", "--dir", "/usr/bin"]);
    process.args(["--symlink", "/bin/env", "/usr/bin/env"]);
    loader_surface.mount(&mut process);
    for path in offered_closure {
        process.args(["--ro-bind", path, path]);
    }
    for credential in credentials {
        let destination = format!("/run/cix-credentials/{}", credential.name);
        process.args(["--dir", "/run", "--dir", "/run/cix-credentials"]);
        process
            .arg("--ro-bind")
            .arg(&credential.source)
            .arg(&destination);
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
    if let Some(credential) = credentials.first() {
        process
            .arg("--setenv")
            .arg("CIX_FETCH_CREDENTIAL_FILE")
            .arg(format!("/run/cix-credentials/{}", credential.name));
        process
            .arg("--setenv")
            .arg("CIX_FETCH_TOKEN")
            .arg(&credential.name);
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
            "starting traced bubblewrap sandbox; this host must permit ptrace and unprivileged user namespaces",
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
        if let Ok(trace_text) = fs::read_to_string(&trace_path) {
            if let Some(hint) =
                fhs::failure_hint(workdir, imports, &trace::parse_failure(&trace_text))
            {
                failure.push('\n');
                failure.push_str(&hint);
            }
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            failure.push_str("\ncommand stderr:\n");
            failure.push_str(stderr.trim());
        }
        bail!("{failure}");
    }
    let trace_text = fs::read_to_string(&trace_path)
        .with_context(|| format!("reading syscall trace {}", trace_path.display()))?;
    Ok(trace::parse(&trace_text))
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
    snapshot_nar_hash: &str,
    volatile: BTreeMap<String, VolatilePath>,
    name: &str,
) -> Result<FetchPin> {
    if expected {
        let mut pin = previous
            .cloned()
            .context("declared EXPECT pin was not installed")?;
        pin.snapshot_nar_hash = snapshot_nar_hash.to_owned();
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
    pin.snapshot_nar_hash = snapshot_nar_hash.to_owned();
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
    use crate::fetch::{
        concrete_fetch_url, revoke_from_store, token_matches, url_prefix, Consent, ConsentStore,
        CredentialToken,
    };

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
            universe_identities: &[],
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
                universe_identities: &[],
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
                universe_identities: &[],
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
                universe_identities: &[],
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
                universe_identities: &[],
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
                universe_identities: &[],
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
            universe_identities: &[],
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
            universe_identities: &[],
        })
        .unwrap();
        assert_ne!(before, after);
        assert_ne!(
            top_fetch_chain_key("fetch", &offered, &environment, "sha256-one", &[]).unwrap(),
            top_fetch_chain_key("fetch", &offered, &environment, "sha256-two", &[]).unwrap()
        );
    }

    #[test]
    fn copy_key_arguments_exclude_physical_directive_provenance() {
        let original = Copy {
            src: Template {
                parts: vec![
                    TemplatePart::Binder {
                        name: "src".into(),
                        line: 8,
                    },
                    TemplatePart::Literal("/rust/".into()),
                ],
            },
            dst: ".".into(),
            mode: crate::CopyMode::Materialize,
            line: 8,
            source: "COPY ${src}/rust/ .".into(),
        };
        let formatted = Copy {
            src: Template {
                parts: vec![
                    TemplatePart::Binder {
                        name: "src".into(),
                        line: 7,
                    },
                    TemplatePart::Literal("/rust/".into()),
                ],
            },
            dst: ".".into(),
            mode: crate::CopyMode::Materialize,
            line: 7,
            source: "  COPY ${src}/rust/ .".into(),
        };

        assert_eq!(
            copy_key_arguments(&original).unwrap(),
            copy_key_arguments(&formatted).unwrap()
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
            universe_identities: &[],
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
            universe_identities: &[],
        })
        .unwrap();
        assert_ne!(one_first, two_first);
    }

    #[test]
    fn ordered_overlay_identity_participates_in_chain_keys() {
        let environment = BTreeMap::new();
        let base = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "true",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            predecessor: "previous",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &["base:one:overlay:a".into()],
        })
        .unwrap();
        let changed_overlay = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "true",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            predecessor: "previous",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &["base:one:overlay:b".into()],
        })
        .unwrap();
        let moved_base = step_key(StepKeyRequest {
            kind: "RUN",
            arguments: "true",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            predecessor: "previous",
            declared_sources: &[],
            environment: &environment,
            fetch_pin: None,
            universe_identities: &["base:two:overlay:a".into()],
        })
        .unwrap();
        assert_ne!(base, changed_overlay);
        assert_ne!(base, moved_base);

        let memo_base = step_memo_key(StepMemoKeyRequest {
            builder: "build",
            index: 0,
            kind: "RUN",
            directive: "RUN true",
            arguments: "true",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            environment: &environment,
            universe_identities: &["base:one:overlay:a".into()],
        })
        .unwrap();
        let memo_changed_overlay = step_memo_key(StepMemoKeyRequest {
            builder: "build",
            index: 0,
            kind: "RUN",
            directive: "RUN true",
            arguments: "true",
            offered_closure: &BTreeSet::new(),
            ordered_imports: &[],
            environment: &environment,
            universe_identities: &["base:one:overlay:b".into()],
        })
        .unwrap();
        assert_ne!(memo_base, memo_changed_overlay);
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
    fn fetch_self_read_requires_the_complete_recorded_write_set() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = root.path().join("snapshot");
        let workspace = root.path().join("workspace");
        fs::create_dir_all(&snapshot).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(snapshot.join("foo"), "first").unwrap();
        fs::write(snapshot.join("bar"), "second").unwrap();
        fs::write(workspace.join("foo"), "first").unwrap();
        let memo = StepMemo {
            key: "fetch-a".into(),
            reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
            output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
            changes: BTreeMap::from([
                ("foo".into(), StepChange::Present),
                ("bar".into(), StepChange::Present),
            ]),
            output_hashes: memo_output_hashes(
                &snapshot,
                &BTreeMap::from([
                    ("foo".into(), StepChange::Present),
                    ("bar".into(), StepChange::Present),
                ]),
            )
            .unwrap(),
        };

        assert!(!validate_step_memo(&memo, &workspace, true, None).unwrap().0);
        assert!(verify_cold_read_set(&memo, &workspace, 1, "FETCH a").is_err());

        fs::write(workspace.join("bar"), "second").unwrap();
        assert!(validate_step_memo(&memo, &workspace, true, None).unwrap().0);
        assert!(
            !validate_step_memo(&memo, &workspace, false, None)
                .unwrap()
                .0
        );

        fs::write(workspace.join("foo"), "drifted").unwrap();
        assert!(!validate_step_memo(&memo, &workspace, true, None).unwrap().0);
    }

    #[test]
    fn self_read_exception_never_crosses_memo_owners() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = root.path().join("snapshot");
        let workspace = root.path().join("workspace");
        fs::create_dir_all(&snapshot).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(snapshot.join("foo"), "a-output").unwrap();
        fs::write(workspace.join("foo"), "a-output").unwrap();
        let a = StepMemo {
            key: "fetch-a".into(),
            reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
            output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
            changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
            output_hashes: memo_output_hashes(
                &snapshot,
                &BTreeMap::from([("foo".into(), StepChange::Present)]),
            )
            .unwrap(),
        };
        let b = StepMemo {
            key: "run-b".into(),
            reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
            output_snapshot: None,
            changes: BTreeMap::new(),
            output_hashes: BTreeMap::new(),
        };

        assert!(validate_step_memo(&a, &workspace, true, None).unwrap().0);
        assert!(!validate_step_memo(&b, &workspace, true, None).unwrap().0);
    }

    #[test]
    fn a_fetch_self_states_never_allow_b_to_bypass_its_own_fingerprint() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = root.path().join("snapshot");
        let workspace = root.path().join("workspace");
        fs::create_dir_all(&snapshot).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(snapshot.join("foo"), "pinned-a-output").unwrap();
        fs::write(workspace.join("foo"), "pinned-a-output").unwrap();
        let a = StepMemo {
            key: "fetch-a".into(),
            reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
            output_snapshot: Some(snapshot.to_string_lossy().into_owned()),
            changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
            output_hashes: memo_output_hashes(
                &snapshot,
                &BTreeMap::from([("foo".into(), StepChange::Present)]),
            )
            .unwrap(),
        };
        let b = StepMemo {
            key: "run-b".into(),
            reads: BTreeMap::from([("foo".into(), crate::ReadDependency::Absent)]),
            output_snapshot: None,
            changes: BTreeMap::new(),
            output_hashes: BTreeMap::new(),
        };

        // a may use its own constructive output; b's read is still checked
        // only against b's recorded fingerprint.
        assert!(validate_step_memo(&a, &workspace, true, None).unwrap().0);
        assert!(!validate_step_memo(&b, &workspace, false, None).unwrap().0);

        // If a executes again and its output moves, b remains a miss and the
        // automatic FETCH pin stays the loud boundary until --update-lock.
        revert_step_writes(&a, &workspace).unwrap();
        fs::write(workspace.join("foo"), "drifted-a-output").unwrap();
        assert!(!validate_step_memo(&a, &workspace, true, None).unwrap().0);
        assert!(!validate_step_memo(&b, &workspace, false, None).unwrap().0);
        let error = verify_fetch_pin(
            Some(&FetchPin::expected("sha256-pinned".into())),
            Some("sha256-drifted"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("--update-lock"), "{error}");
    }

    #[test]
    fn executing_fetch_reverts_its_superseded_writes_before_tracing() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("foo"), "old fetch output").unwrap();
        fs::write(root.path().join("kept"), "not fetch-owned").unwrap();
        let memo = StepMemo {
            key: "fetch-a".into(),
            reads: BTreeMap::new(),
            output_snapshot: None,
            changes: BTreeMap::from([("foo".into(), StepChange::Present)]),
            output_hashes: BTreeMap::new(),
        };

        revert_step_writes(&memo, root.path()).unwrap();
        assert!(!root.path().join("foo").exists());
        assert_eq!(
            fs::read_to_string(root.path().join("kept")).unwrap(),
            "not fetch-owned"
        );
    }

    #[test]
    fn fetch_records_a_new_output_tree_as_one_constructive_root() {
        let before = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir_all(workspace.path().join("vendor/nested")).unwrap();
        fs::write(workspace.path().join("vendor/first"), "one").unwrap();
        fs::write(workspace.path().join("vendor/nested/second"), "two").unwrap();
        let mut changes = BTreeMap::from([
            ("vendor/first".into(), StepChange::Present),
            ("vendor/nested/second".into(), StepChange::Present),
        ]);

        retain_fetch_output_roots(before.path(), workspace.path(), &mut changes).unwrap();
        assert_eq!(
            changes,
            BTreeMap::from([("vendor".into(), StepChange::Present)])
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
            .and_then(|candidate| candidate.canonicalize().ok())
            .expect("the Nix test host provides a resolvable bash");
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
            &[],
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(work.path().join("result")).unwrap(),
            "fallback-ok"
        );
    }

    #[test]
    fn fetch_credential_matching_uses_concrete_url_prefixes() {
        assert_eq!(
            url_prefix("https://packages.example.test/team/npm/pkg.tgz").unwrap(),
            "https://packages.example.test/team"
        );
        assert!(token_matches(
            "https://packages.example.test/team/*",
            "https://packages.example.test/team/npm/pkg.tgz"
        ));
        assert!(!token_matches(
            "https://packages.example.test/other/*",
            "https://packages.example.test/team/npm/pkg.tgz"
        ));
        assert_eq!(
            concrete_fetch_url("curl --fail 'https://packages.example.test/team/npm/pkg.tgz'"),
            Some("https://packages.example.test/team/npm/pkg.tgz".into())
        );
    }

    #[test]
    fn fetch_consent_is_scoped_to_project_prefix_and_token() {
        let project = PathBuf::from("/work/example");
        let first = Consent {
            project: project.clone(),
            token: "packages".into(),
            prefix: "https://packages.example.test/team".into(),
        };
        let second_prefix = Consent {
            project: project.clone(),
            token: "packages".into(),
            prefix: "https://packages.example.test/other".into(),
        };
        let other_project = Consent {
            project: PathBuf::from("/work/other"),
            token: "packages".into(),
            prefix: first.prefix.clone(),
        };
        let mut store = ConsentStore {
            grants: BTreeSet::from([first.clone(), second_prefix, other_project]),
        };

        assert!(store.grants.contains(&first));
        assert_eq!(revoke_from_store(&mut store, "packages"), 3);
        assert!(store.grants.is_empty());
    }

    #[test]
    fn removed_fetch_token_refuses_an_anonymous_retry() {
        let project = PathBuf::from("/work/example");
        let mut credentials = HostCredentials {
            project: project.clone(),
            tokens: BTreeMap::new(),
            consent_path: PathBuf::from("/tmp/fetch-consents.json"),
            consent: ConsentStore {
                grants: BTreeSet::from([Consent {
                    project,
                    token: "retired".into(),
                    prefix: "https://packages.example.test/team".into(),
                }]),
            },
            allow_secret: true,
        };

        let error = match credentials.for_command("curl https://packages.example.test/team/pkg.tgz")
        {
            Ok(_) => panic!("a removed token must not allow an anonymous FETCH"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("retired"), "{error}");
        assert!(error.contains("refusing anonymous retry"), "{error}");
    }

    #[test]
    fn a_new_fetch_prefix_needs_its_own_consent() {
        let directory = tempfile::tempdir().unwrap();
        let credential = directory.path().join("credential");
        fs::write(&credential, "not logged").unwrap();
        let project = PathBuf::from("/work/example");
        let old = Consent {
            project: project.clone(),
            token: "packages".into(),
            prefix: "https://packages.example.test/team".into(),
        };
        let mut credentials = HostCredentials {
            project: project.clone(),
            tokens: BTreeMap::from([(
                "packages".into(),
                CredentialToken {
                    url: "https://packages.example.test/*".into(),
                    credential,
                },
            )]),
            consent_path: directory.path().join("consent.json"),
            consent: ConsentStore {
                grants: BTreeSet::from([old]),
            },
            allow_secret: true,
        };

        let mounted = credentials
            .for_command("curl https://packages.example.test/other/pkg.tgz")
            .unwrap()
            .expect("matching token is available");
        assert_eq!(mounted.name, "packages");
        assert!(credentials
            .consent
            .grants
            .iter()
            .all(|grant| grant.prefix != "https://packages.example.test/other"));
    }
}
