//! Nix-evaluation request/result boundary for build execution.
//!
//! The conductor asks this owner for a typed FETCH or BUILDER answer. It never
//! shares the raw Nix JSON context across the evaluation seam.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Cixfile, DevEnvironment, LockFile};

pub trait EvaluationCodegen {
    fn fetch_context(
        &self,
        cixfile: &Cixfile,
        fetch_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String>;

    fn fetch_offers(
        &self,
        cixfile: &Cixfile,
        fetch_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String>;

    fn builder_context(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String>;

    fn builder_offers(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String>;

    fn builder_dev_environment(
        &self,
        cixfile: &Cixfile,
        builder_name: &str,
        source_dir: &Path,
        lock: &LockFile,
        system: &str,
        snapshots: &BTreeMap<String, String>,
    ) -> Result<String>;
}

#[derive(Clone, Debug)]
pub(crate) struct FetchContext {
    pub(crate) offers: Vec<String>,
    pub(crate) command: String,
    pub(crate) imports: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) universe_identities: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) struct BuilderContext {
    pub(crate) offers: Vec<String>,
    pub(crate) commands: Vec<String>,
    pub(crate) copies: Vec<String>,
    pub(crate) imports: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) universe_identities: BTreeMap<String, String>,
}

pub(crate) struct FetchContextRequest<'a> {
    pub(crate) cixfile: &'a Cixfile,
    pub(crate) name: &'a str,
    pub(crate) directory: &'a Path,
    pub(crate) lock: &'a LockFile,
    pub(crate) system: &'a str,
    pub(crate) snapshots: &'a BTreeMap<String, String>,
}

pub(crate) struct BuilderContextRequest<'a> {
    pub(crate) cixfile: &'a Cixfile,
    pub(crate) name: &'a str,
    pub(crate) directory: &'a Path,
    pub(crate) lock: &'a LockFile,
    pub(crate) system: &'a str,
    pub(crate) snapshots: &'a BTreeMap<String, String>,
}

pub(crate) struct DevEnvironmentRequest<'a> {
    pub(crate) cixfile: &'a Cixfile,
    pub(crate) builder_name: &'a str,
    pub(crate) directory: &'a Path,
    pub(crate) lock: &'a mut LockFile,
    pub(crate) system: &'a str,
    pub(crate) snapshots: &'a BTreeMap<String, String>,
    pub(crate) imports: &'a [String],
    pub(crate) universe_identities: &'a BTreeMap<String, String>,
}

pub(crate) struct NixEvaluation;

impl NixEvaluation {
    pub(crate) fn fetch_context(
        codegen: &dyn EvaluationCodegen,
        request: FetchContextRequest<'_>,
    ) -> Result<FetchContext> {
        let expression = codegen.fetch_context(
            request.cixfile,
            request.name,
            request.directory,
            request.lock,
            request.system,
            request.snapshots,
        )?;
        let raw = evaluate_context(&expression)?;
        if raw.commands.len() != 1 {
            bail!(
                "internal top-level FETCH context mismatch: resolved {} commands",
                raw.commands.len()
            );
        }
        Ok(FetchContext {
            offers: raw.offers,
            command: raw
                .commands
                .into_iter()
                .next()
                .expect("checked command count"),
            imports: raw.imports,
            environment: raw.environment,
            universe_identities: raw.universe_identities,
        })
    }

    pub(crate) fn builder_context(
        codegen: &dyn EvaluationCodegen,
        request: BuilderContextRequest<'_>,
    ) -> Result<BuilderContext> {
        let expression = codegen.builder_context(
            request.cixfile,
            request.name,
            request.directory,
            request.lock,
            request.system,
            request.snapshots,
        )?;
        let raw = cached_context(&expression, request.directory)?
            .map(Ok)
            .unwrap_or_else(|| evaluate_context(&expression))?;
        cache_context(&expression, request.directory, &raw)?;
        Ok(BuilderContext {
            offers: raw.offers,
            commands: raw.commands,
            copies: raw.copies,
            imports: raw.imports,
            environment: raw.environment,
            universe_identities: raw.universe_identities,
        })
    }

    pub(crate) fn realize_fetch_offers(
        codegen: &dyn EvaluationCodegen,
        request: FetchContextRequest<'_>,
    ) -> Result<()> {
        let expression = codegen.fetch_offers(
            request.cixfile,
            request.name,
            request.directory,
            request.lock,
            request.system,
            request.snapshots,
        )?;
        realize_offers(&expression)
    }

    pub(crate) fn realize_builder_offers(
        codegen: &dyn EvaluationCodegen,
        request: BuilderContextRequest<'_>,
    ) -> Result<()> {
        let expression = codegen.builder_offers(
            request.cixfile,
            request.name,
            request.directory,
            request.lock,
            request.system,
            request.snapshots,
        )?;
        realize_offers(&expression)
    }

    pub(crate) fn offered_closure(offers: &[String]) -> Result<BTreeSet<String>> {
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
        let stdout =
            String::from_utf8(output.stdout).context("nix-store returned non-UTF-8 paths")?;
        Ok(stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    pub(crate) fn development_environment(
        codegen: &dyn EvaluationCodegen,
        request: DevEnvironmentRequest<'_>,
    ) -> Result<BTreeMap<String, String>> {
        if request.imports.is_empty() {
            return Ok(BTreeMap::new());
        }
        let universe = request
            .cixfile
            .inputs
            .iter()
            .find(|(_, input)| input.kind == crate::InputKind::PackageUniverse)
            .map(|(name, _)| name)
            .context("BUILDER IMPORT needs a package-universe FROM")?;
        let identity = request
            .universe_identities
            .get(universe)
            .context("package universe identity was not resolved")?;
        let key = format!(
            "{identity}:{}",
            hex_hash(request.imports.join("\0").as_bytes())
        );
        if let Some(snapshot) = request.lock.dev_envs.get(&key) {
            request
                .lock
                .builder_dev_envs
                .insert(request.builder_name.to_owned(), key.clone());
            let environment = filter_development_environment(&snapshot.environment);
            if environment != snapshot.environment {
                request.lock.dev_envs.insert(
                    key,
                    DevEnvironment {
                        environment: environment.clone(),
                    },
                );
            }
            return Ok(environment);
        }
        let expression = codegen.builder_dev_environment(
            request.cixfile,
            request.builder_name,
            request.directory,
            request.lock,
            request.system,
            request.snapshots,
        )?;
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
        request.lock.dev_envs.insert(
            key.clone(),
            DevEnvironment {
                environment: environment.clone(),
            },
        );
        request
            .lock
            .builder_dev_envs
            .insert(request.builder_name.to_owned(), key);
        Ok(environment)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawContext {
    offers: Vec<String>,
    imports: Vec<String>,
    commands: Vec<String>,
    copies: Vec<String>,
    environment: BTreeMap<String, String>,
    #[serde(rename = "universeIdentities")]
    universe_identities: BTreeMap<String, String>,
}

/// The generated context expression is byte-stable across source edits (the
/// source enters it only as a `builtins.path` literal of a fixed directory
/// path), so its evaluation result is reusable as long as every resolved
/// store path still exists — except `copies` entries rooted in the source,
/// which move with the source content. Those are re-rooted by store-adding
/// the source directory (`nix store add --mode nar` computes the identical
/// path to `builtins.path`). Expressions whose results depend on source
/// content beyond that root (hashFile interpolations, project-local overlays)
/// never take this fastpath.
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
    context: RawContext,
}

fn cached_context(expression: &str, directory: &Path) -> Result<Option<RawContext>> {
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

fn cache_context(expression: &str, directory: &Path, context: &RawContext) -> Result<()> {
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
    let payload = serde_json::to_vec_pretty(&CachedContext {
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

fn evaluate_context(expression: &str) -> Result<RawContext> {
    let raw = cix_common::nix(&["eval", "--impure", "--json", "--expr", expression])
        .context("resolving RUN/FETCH build context from locked FROM inputs")?;
    serde_json::from_str(&raw).context("parsing resolved RUN/FETCH build context")
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

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
