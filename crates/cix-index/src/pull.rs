//! Remote index resolution and substitution-aware pulling.

use super::refs::*;
use super::tags::tag;
use anyhow::{anyhow, bail, Context, Result};
use cix_common::{current_system, nix, Ref};
use std::path::Path;

fn endpoint(reference: &Ref, path: &str) -> Result<String> {
    let root = reference
        .root_url
        .as_deref()
        .context("pull requires a remote root_url")?;
    let scheme = if root.starts_with("localhost") || root.starts_with("127.") {
        "http"
    } else {
        "https"
    };
    Ok(format!("{scheme}://{root}{path}"))
}

pub(crate) fn resolve_remote(reference: &Ref) -> Result<Entry> {
    let url = endpoint(reference, &format!("/{}:{}", reference.name, reference.tag))?;
    let response = ureq::get(&url)
        .set("Accept", "application/vnd.cix+json;version=1")
        .call()
        .map_err(|error| anyhow!("resolving {url}: {error}"))?;
    if response.status() != 200 {
        bail!("resolving {url} returned HTTP {}", response.status());
    }
    response
        .into_json()
        .context("parsing index resolve response")
}

pub(crate) fn fetch_output(reference: &Ref, entry: &Entry, output: &Output) -> Result<()> {
    if Path::new(&output.store_path).exists() {
        let actual = path_info(&output.store_path)?;
        if actual.nar_hash != output.nar_hash {
            bail!(
                "narHash mismatch for {}: index has {}, local store has {}",
                output.store_path,
                output.nar_hash,
                actual.nar_hash
            );
        }
        return Ok(());
    }
    if entry.substituters.is_empty() {
        bail!(
            "remote `{}` did not advertise a substituter",
            reference.display()
        );
    }
    let mut failures = Vec::new();
    for substituter in &entry.substituters {
        let trusted_keys = entry.trusted_keys.join(" ");
        let mut arguments = vec!["copy", "--from", substituter.as_str()];
        if !trusted_keys.is_empty() {
            arguments.extend(["--option", "trusted-public-keys", &trusted_keys]);
        }
        arguments.push(&output.store_path);
        match nix(&arguments) {
            Ok(_) => {
                let actual = path_info(&output.store_path)?;
                if actual.nar_hash != output.nar_hash {
                    bail!(
                        "narHash mismatch for {}: index has {}, local store has {}",
                        output.store_path,
                        output.nar_hash,
                        actual.nar_hash
                    );
                }
                return Ok(());
            }
            Err(error) => failures.push(format!("{substituter}: {error:#}")),
        }
    }
    bail!(
        "could not fetch {} from any substituter: {}",
        output.store_path,
        failures.join("; ")
    )
}

/// Resolve a store path, local tag, or qualified index ref for the current system.
///
/// Qualified refs are resolved directly against the index and fetched from an advertised
/// substituter when necessary. Unlike [`pull`], this does not create a local mirror tag.
pub fn resolve_with(store: &Store, reference: &str) -> Result<Output> {
    if reference.starts_with("/nix/store/") {
        return path_info(reference);
    }
    let reference = Ref::parse(reference)?;
    let system = current_system()?;
    if reference.root_url.is_some() {
        let entry = resolve_remote(&reference)?;
        let output = entry.outputs.get(&system).cloned().with_context(|| {
            format!(
                "remote `{}` has no output for {system}",
                reference.display()
            )
        })?;
        fetch_output(&reference, &entry, &output)?;
        return Ok(output);
    }
    let metadata = store
        .load(&reference)?
        .with_context(|| format!("local tag `{}` does not exist", reference.display()))?;
    let output = metadata
        .entry
        .outputs
        .get(&system)
        .cloned()
        .with_context(|| {
            format!(
                "local tag `{}` has no output for {system}",
                reference.display()
            )
        })?;
    fetch_output(&reference, &metadata.entry, &output)?;
    Ok(output)
}

pub fn resolve(reference: &str) -> Result<Output> {
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/state"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".local/state"))
        .join("cix");
    resolve_with(&Store::open(root)?, reference)
}

fn pull_one(store: &Store, remote: &Ref, local: &Ref) -> Result<bool> {
    let entry = resolve_remote(remote)?;
    let system = current_system()?;
    let output = entry
        .outputs
        .get(&system)
        .with_context(|| format!("remote `{}` has no output for {system}", remote.display()))?;
    if store
        .load(local)?
        .and_then(|metadata| metadata.entry.outputs.get(&system).cloned())
        .is_some_and(|existing| existing.nar_hash == output.nar_hash)
    {
        return Ok(false);
    }
    fetch_output(remote, &entry, output)?;
    tag(
        store,
        &output.store_path,
        &local.display(),
        remote.root_url.clone(),
    )?;
    Ok(true)
}

pub fn pull(store: &Store, reference: Option<&str>, as_ref: Option<&str>) -> Result<usize> {
    match reference {
        Some(input) => {
            let remote = Ref::parse(input)?;
            if remote.root_url.is_none() {
                bail!("pull requires a fully-qualified ref with a root_url");
            }
            let local = match as_ref {
                Some(alias) => Ref::parse(alias)?,
                None => remote.clone(),
            };
            Ok(usize::from(pull_one(store, &remote, &local)?))
        }
        None => {
            if as_ref.is_some() {
                bail!("--as requires a remote ref");
            }
            let mut changed = 0;
            for metadata in store.all()? {
                let Some(upstream) = metadata.upstream else {
                    continue;
                };
                let local = Ref::parse(&metadata.reference)?;
                let mut remote = local.clone();
                remote.root_url = Some(upstream);
                changed += usize::from(pull_one(store, &remote, &local)?);
            }
            Ok(changed)
        }
    }
}
