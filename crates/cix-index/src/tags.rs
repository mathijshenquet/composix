//! Immutable tag-table publication, inspection, and history.

use super::pull::{fetch_output, resolve_remote};
use super::refs::*;
use anyhow::{bail, Context, Result};
use cix_common::{build_installable, current_system, nix, Ref};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn tag(store: &Store, installable: &str, target: &str, upstream: Option<String>) -> Result<()> {
    let reference = Ref::parse(target)?;
    if reference.root_url.is_some() && upstream.is_none() {
        bail!(
           "qualified names denote remote state; tags are bare. To publish, tag on the box that serves (see docs/design.md \"The org workflow\")."
       );
    }
    let path = resolved_path(store, installable)?;
    let output = path_info(&path)?;
    let system = current_system()?;
    let mut metadata = store.load(&reference)?.unwrap_or_else(|| TagMetadata {
        reference: reference.display(),
        entry: Entry {
            outputs: BTreeMap::new(),
            substituters: Vec::new(),
            trusted_keys: Vec::new(),
            created_at: timestamp(),
        },
        upstream: upstream.clone(),
    });
    metadata.reference = reference.display();
    metadata.entry.outputs.insert(system, output.clone());
    if metadata.upstream.is_none() {
        metadata.upstream = upstream;
    }
    store.publish(&reference.name, &reference.tag, metadata)
}

pub fn untag(store: &Store, target: &str) -> Result<()> {
    let reference = Ref::parse(target)?;
    if !store.yank(&reference.name, &reference.tag)? {
        bail!("tag `{}` does not exist", reference.display());
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub nar_hash: String,
    pub tags: Vec<String>,
}

impl Store {
    fn table_for_nar_hash(&self, nar_hash: &str) -> Result<Option<TablePointer>> {
        let raw = nix(&["path-info", "--all", "--json", "--json-format", "1"])?;
        let infos: BTreeMap<String, PathInfo> =
            serde_json::from_str(&raw).context("parsing nix path-info --all JSON")?;
        Ok(infos.into_iter().find_map(|(store_path, info)| {
            (info.nar_hash == nar_hash && Path::new(&store_path).join("table.json").is_file())
                .then_some(TablePointer {
                    store_path,
                    nar_hash: info.nar_hash,
                })
        }))
    }

    /// Walk the currently available immutable table chain for one name.
    pub fn history(&self, name: &str) -> Result<Vec<HistoryEntry>> {
        let mut pointer = match self.read_pointer(name)? {
            Some(pointer) => pointer,
            None => return Ok(Vec::new()),
        };
        let mut history = Vec::new();
        loop {
            let table = match self.read_table(&pointer) {
                Ok(table) => table,
                Err(error)
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
                {
                    break
                }
                Err(error) => return Err(error),
            };
            if table.name != name {
                bail!(
                    "history pointer for `{name}` points to table for `{}`",
                    table.name
                );
            }
            history.push(HistoryEntry {
                nar_hash: pointer.nar_hash.clone(),
                tags: table.tags.keys().cloned().collect(),
            });
            let Some(parent) = table.parent else { break };
            let Some(next) = self.table_for_nar_hash(&parent)? else {
                break;
            };
            pointer = next;
        }
        Ok(history)
    }
}

pub fn history(store: &Store, name: &str) -> Result<Vec<HistoryEntry>> {
    store.history(name)
}

pub fn list(store: &Store, prefix: Option<&str>, long: bool) -> Result<String> {
    let system = current_system()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(render_list(store.all()?, prefix, long, &system, now))
}

pub(crate) fn render_list(
    tags: Vec<TagMetadata>,
    prefix: Option<&str>,
    long: bool,
    system: &str,
    now: u64,
) -> String {
    let rows = tags
        .into_iter()
        .filter(|tag| prefix.is_none_or(|prefix| tag.reference.starts_with(prefix)))
        .map(|tag| {
            if !long {
                return vec![tag.reference];
            }
            let systems = tag
                .entry
                .outputs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(",");
            let path = tag
                .entry
                .outputs
                .get(system)
                .map(|output| output.store_path.as_str())
                .unwrap_or("-");
            let age = tag
                .entry
                .created_at
                .parse::<u64>()
                .ok()
                .map(|created| format!("{}s", now.saturating_sub(created)))
                .unwrap_or_else(|| "unknown".into());
            vec![
                tag.reference,
                systems,
                path.to_owned(),
                tag.upstream.unwrap_or_else(|| "-".into()),
                age,
            ]
        })
        .collect::<Vec<_>>();
    if !long {
        return rows
            .into_iter()
            .filter_map(|row| row.into_iter().next())
            .collect::<Vec<_>>()
            .join("\n");
    }
    let headers = ["REF", "SYSTEMS", "PATH", "UPSTREAM", "AGE"];
    let widths = (0..headers.len())
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(String::len)
                .max()
                .unwrap_or_default()
                .max(headers[column].len())
        })
        .collect::<Vec<_>>();
    let render = |row: &[String]| {
        row.iter()
            .enumerate()
            .map(|(column, value)| format!("{value:<width$}", width = widths[column]))
            .collect::<Vec<_>>()
            .join("  ")
    };
    let mut lines = vec![render(&headers.map(str::to_owned))];
    lines.extend(rows.iter().map(|row| render(row)));
    lines.join("\n")
}

/// Resolve an artifact without discarding the index entry that named it.
pub fn inspect_artifact(store: &Store, installable: &str) -> Result<Artifact> {
    if installable.starts_with("/nix/store/") {
        return Ok(Artifact {
            output: path_info(installable)?,
            metadata: None,
        });
    }

    if let Ok(reference) = Ref::parse(installable) {
        if reference.root_url.is_some() {
            let entry = resolve_remote(&reference)?;
            let system = current_system()?;
            let output = entry.outputs.get(&system).cloned().with_context(|| {
                format!(
                    "remote `{}` has no output for {system}",
                    reference.display()
                )
            })?;
            fetch_output(&reference, &entry, &output)?;
            return Ok(Artifact {
                output,
                metadata: Some(TagMetadata {
                    reference: reference.display(),
                    entry,
                    upstream: reference.root_url,
                }),
            });
        }
        if let Some(metadata) = store.load(&reference)? {
            let system = current_system()?;
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
            return Ok(Artifact {
                output,
                metadata: Some(metadata),
            });
        }
    }

    let path = build_installable(installable)?;
    Ok(Artifact {
        output: path_info(&path)?,
        metadata: None,
    })
}

/// Return the closure size when the host's Nix supports `path-info -S`.
pub fn closure_size(store_path: &str) -> Option<u64> {
    nix(&["path-info", "-S", store_path])
        .ok()
        .and_then(|output| {
            output
                .split_whitespace()
                .last()
                .and_then(|size| size.parse().ok())
        })
}
