//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See docs/design.md "Part 1 — index".

pub mod cli;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Read,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use cix_common::{build_installable, current_system, nix, Ref};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub store_path: String,
    pub nar_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drv_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub outputs: BTreeMap<String, Output>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub substituters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_keys: Vec<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagMetadata {
    pub reference: String,
    #[serde(flatten)]
    pub entry: Entry,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagRecord {
    store_path: String,
    nar_hash: String,
    meta: TagMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagTable {
    cix_tag_table: u8,
    name: String,
    parent: Option<String>,
    tags: BTreeMap<String, TagRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TablePointer {
    store_path: String,
    nar_hash: String,
}

#[derive(Debug, thiserror::Error)]
#[error("name pointer changed concurrently; retry the publish")]
pub struct PointerChanged;

/// The artifact-facing data behind `cix inspect`.
#[derive(Clone, Debug)]
pub struct Artifact {
    pub output: Output,
    pub metadata: Option<TagMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathInfo {
    nar_hash: String,
    #[serde(default)]
    deriver: Option<String>,
}

/// The on-disk user index. A mutable name pointer selects an immutable tag table.
#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open() -> Result<Self> {
        let root = match env::var_os("CIX_STATE_DIR") {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(env::var_os("HOME").context("HOME is unset; set CIX_STATE_DIR")?)
                .join(".local/state/cix"),
        };
        Self::open_at(root)
    }

    fn open_at(root: PathBuf) -> Result<Self> {
        let store = Self { root };
        fs::create_dir_all(store.roots_dir()).context("creating cix roots directory")?;
        fs::create_dir_all(store.names_dir()).context("creating cix names directory")?;
        fs::create_dir_all(store.tmp_dir()).context("creating cix temporary directory")?;
        store.migrate_legacy()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn roots_dir(&self) -> PathBuf {
        self.root.join("roots")
    }

    fn names_dir(&self) -> PathBuf {
        self.root.join("names")
    }

    fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn legacy_meta_dir(&self) -> PathBuf {
        self.root.join("tags")
    }

    fn legacy_dir(&self) -> PathBuf {
        self.root.join("meta.legacy")
    }

    pub fn encode(reference: &Ref) -> String {
        URL_SAFE_NO_PAD.encode(reference.display())
    }

    pub fn encode_name(name: &str) -> String {
        URL_SAFE_NO_PAD.encode(name)
    }

    fn pointer_path(&self, name: &str) -> PathBuf {
        self.names_dir().join(Self::encode_name(name))
    }

    fn pointer_lock_path(&self) -> PathBuf {
        self.names_dir().join(".lock")
    }

    fn name_roots_dir(&self, name: &str) -> PathBuf {
        self.roots_dir().join("names").join(Self::encode_name(name))
    }

    fn read_pointer(&self, name: &str) -> Result<Option<TablePointer>> {
        let path = self.pointer_path(name);
        match fs::read(&path) {
            Ok(contents) => serde_json::from_slice(&contents)
                .with_context(|| format!("parsing name pointer {}", path.display()))
                .map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("reading name pointer {}", path.display()))
            }
        }
    }

    fn write_pointer(&self, name: &str, pointer: &TablePointer) -> Result<()> {
        let path = self.pointer_path(name);
        let temporary = self.names_dir().join(format!(
            ".{}.{}.tmp",
            Self::encode_name(name),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::write(&temporary, serde_json::to_vec_pretty(pointer)?)
            .with_context(|| format!("writing name pointer {}", temporary.display()))?;
        fs::rename(&temporary, &path).context("atomically replacing name pointer")
    }

    fn with_pointer_lock<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.pointer_lock_path())
            .context("opening name pointer lock")?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(std::io::Error::last_os_error()).context("locking name pointers");
        }
        let result = operation();
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_UN) } != 0 {
            return Err(std::io::Error::last_os_error()).context("unlocking name pointers");
        }
        result
    }

    fn read_table(&self, pointer: &TablePointer) -> Result<TagTable> {
        let path = Path::new(&pointer.store_path).join("table.json");
        let table: TagTable = serde_json::from_slice(
            &fs::read(&path).with_context(|| format!("reading tag table {}", path.display()))?,
        )
        .with_context(|| format!("parsing tag table {}", path.display()))?;
        if table.cix_tag_table != 1 {
            bail!("unsupported cix tag table version {}", table.cix_tag_table);
        }
        Ok(table)
    }

    fn current_table(&self, name: &str) -> Result<(Option<TablePointer>, TagTable)> {
        let pointer = self.read_pointer(name)?;
        let table = match &pointer {
            Some(pointer) => self.read_table(pointer)?,
            None => TagTable {
                cix_tag_table: 1,
                name: name.to_owned(),
                parent: None,
                tags: BTreeMap::new(),
            },
        };
        if table.name != name {
            bail!(
                "name pointer for `{name}` points to table for `{}`",
                table.name
            );
        }
        Ok((pointer, table))
    }

    fn add_table(&self, table: &TagTable) -> Result<TablePointer> {
        let parent = self.tmp_dir().join(format!(
            "table-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&parent).with_context(|| format!("creating {}", parent.display()))?;
        let directory = parent.join("table");
        fs::create_dir(&directory).with_context(|| format!("creating {}", directory.display()))?;
        let result = (|| {
            let mut json = serde_json::to_vec_pretty(table)?;
            json.push(b'\n');
            fs::write(directory.join("table.json"), json)?;
            let directory_text = directory.to_string_lossy().into_owned();
            let store_path = nix(&["store", "add-path", &directory_text])?
                .trim()
                .to_owned();
            let output = path_info(&store_path)?;
            Ok(TablePointer {
                store_path: output.store_path,
                nar_hash: output.nar_hash,
            })
        })();
        fs::remove_dir_all(&parent).with_context(|| format!("removing {}", parent.display()))?;
        result
    }

    fn replace_root(&self, link: &Path, store_path: &str) -> Result<()> {
        if fs::symlink_metadata(link).is_ok() {
            fs::remove_file(link)
                .with_context(|| format!("replacing GC root {}", link.display()))?;
        }
        let parent = link.parent().context("GC root has no parent")?;
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        let link_text = link.to_string_lossy().into_owned();
        nix(&["build", store_path, "--out-link", &link_text])?;
        Ok(())
    }

    fn referenced_paths(table: &TagTable) -> BTreeSet<String> {
        let mut paths = BTreeSet::new();
        for record in table.tags.values() {
            paths.insert(record.store_path.clone());
            paths.extend(
                record
                    .meta
                    .entry
                    .outputs
                    .values()
                    .map(|output| output.store_path.clone()),
            );
        }
        paths
    }

    fn sync_roots(&self, table: &TagTable, pointer: &TablePointer) -> Result<()> {
        let directory = self.name_roots_dir(&table.name);
        self.replace_root(&directory.join("table"), &pointer.store_path)?;
        let paths = directory.join("paths");
        fs::create_dir_all(&paths)?;
        let wanted = Self::referenced_paths(table)
            .into_iter()
            .map(|path| (Self::encode_name(&path), path))
            .collect::<BTreeMap<_, _>>();
        for (encoded, path) in &wanted {
            self.replace_root(&paths.join(encoded), path)?;
        }
        for entry in fs::read_dir(&paths)? {
            let entry = entry?;
            if !wanted.contains_key(&entry.file_name().to_string_lossy().into_owned()) {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    fn cas_pointer(
        &self,
        name: &str,
        expected: Option<&TablePointer>,
        next: &TablePointer,
        table: &TagTable,
    ) -> Result<()> {
        self.with_pointer_lock(|| {
            if self.read_pointer(name)?.as_ref() != expected {
                return Err(PointerChanged.into());
            }
            // D45: auth = may-move-this-name, enforced at the serve/publish boundary when it exists.
            self.write_pointer(name, next)?;
            self.sync_roots(table, next)
        })
    }

    fn record_from_metadata(metadata: TagMetadata) -> Result<TagRecord> {
        let output = metadata
            .entry
            .outputs
            .values()
            .next()
            .context("tag metadata has no outputs")?;
        Ok(TagRecord {
            store_path: output.store_path.clone(),
            nar_hash: output.nar_hash.clone(),
            meta: metadata,
        })
    }

    /// Publish one tag by atomically moving its name pointer.
    pub fn publish(&self, name: &str, tag: &str, metadata: TagMetadata) -> Result<()> {
        self.publish_many(name, vec![(tag.to_owned(), metadata)])
    }

    /// Publish several tags in one immutable table flip.
    pub fn publish_many(&self, name: &str, tags: Vec<(String, TagMetadata)>) -> Result<()> {
        if tags.is_empty() {
            return Ok(());
        }
        for (tag, _) in &tags {
            let reference = Ref::parse(&format!("{name}:{tag}"))?;
            if reference.root_url.is_some() || reference.name != name || reference.tag != *tag {
                bail!("tag table names must be bare `name:tag` refs");
            }
        }
        for attempt in 0..2 {
            let (expected, mut table) = self.current_table(name)?;
            table.parent = expected.as_ref().map(|pointer| pointer.nar_hash.clone());
            for (tag, metadata) in tags.iter().cloned() {
                table
                    .tags
                    .insert(tag, Self::record_from_metadata(metadata)?);
            }
            let pointer = self.add_table(&table)?;
            match self.cas_pointer(name, expected.as_ref(), &pointer, &table) {
                Err(error) if error.downcast_ref::<PointerChanged>().is_some() && attempt == 0 => {}
                result => return result,
            }
        }
        unreachable!("two-attempt publish loop always returns")
    }

    /// Remove a tag from the current table while retaining the name and its history chain.
    pub fn yank(&self, name: &str, tag: &str) -> Result<bool> {
        for attempt in 0..2 {
            let (expected, mut table) = self.current_table(name)?;
            if !table.tags.contains_key(tag) {
                return Ok(false);
            }
            table.parent = expected.as_ref().map(|pointer| pointer.nar_hash.clone());
            table.tags.remove(tag);
            let pointer = self.add_table(&table)?;
            match self.cas_pointer(name, expected.as_ref(), &pointer, &table) {
                Ok(()) => return Ok(true),
                Err(error) if error.downcast_ref::<PointerChanged>().is_some() && attempt == 0 => {}
                Err(error) => return Err(error),
            }
        }
        unreachable!("two-attempt yank loop always returns")
    }

    /// Delete a name pointer and all of its roots. Historical table items remain GC-managed.
    pub fn remove_name(&self, name: &str) -> Result<bool> {
        self.with_pointer_lock(|| {
            let pointer = self.pointer_path(name);
            let existed = match fs::remove_file(&pointer) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => {
                    return Err(error).with_context(|| format!("removing {}", pointer.display()))
                }
            };
            let roots = self.name_roots_dir(name);
            if roots.exists() {
                fs::remove_dir_all(roots)?;
            }
            Ok(existed)
        })
    }

    pub fn load(&self, reference: &Ref) -> Result<Option<TagMetadata>> {
        let (_, table) = self.current_table(&reference.name)?;
        Ok(table
            .tags
            .get(&reference.tag)
            .map(|record| record.meta.clone()))
    }

    pub fn all(&self) -> Result<Vec<TagMetadata>> {
        let mut tags = Vec::new();
        for entry in fs::read_dir(self.names_dir()).context("listing name pointers")? {
            let entry = entry?;
            if entry.file_name() == ".lock" || !entry.file_type()?.is_file() {
                continue;
            }
            let encoded = entry.file_name().to_string_lossy().into_owned();
            let name = String::from_utf8(URL_SAFE_NO_PAD.decode(encoded)?)
                .context("decoding name pointer")?;
            let (_, table) = self.current_table(&name)?;
            for record in table.tags.into_values() {
                tags.push(record.meta);
            }
        }
        tags.sort_by(|left, right| left.reference.cmp(&right.reference));
        Ok(tags)
    }

    fn migrate_legacy(&self) -> Result<()> {
        let legacy = self.legacy_meta_dir();
        if !legacy.is_dir() {
            return Ok(());
        }
        let mut names: BTreeMap<String, BTreeMap<String, TagRecord>> = BTreeMap::new();
        let mut old_roots = Vec::new();
        for entry in fs::read_dir(&legacy).context("listing legacy tag sidecars")? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("json")
            {
                continue;
            }
            let metadata: TagMetadata = serde_json::from_slice(&fs::read(entry.path())?)
                .with_context(|| format!("parsing legacy sidecar {}", entry.path().display()))?;
            let reference = Ref::parse(&metadata.reference)?;
            names
                .entry(reference.name.clone())
                .or_default()
                .insert(reference.tag.clone(), Self::record_from_metadata(metadata)?);
            old_roots.push(
                self.roots_dir()
                    .join(URL_SAFE_NO_PAD.encode(reference.display())),
            );
        }
        for (name, tags) in names {
            if self.read_pointer(&name)?.is_some() {
                continue;
            }
            let table = TagTable {
                cix_tag_table: 1,
                name: name.clone(),
                parent: None,
                tags,
            };
            let pointer = self.add_table(&table)?;
            self.cas_pointer(&name, None, &pointer, &table)?;
        }
        for root in old_roots {
            if fs::symlink_metadata(&root).is_ok() {
                fs::remove_file(root)?;
            }
        }
        fs::rename(&legacy, self.legacy_dir()).context("moving legacy sidecars to meta.legacy")?;
        Ok(())
    }
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs()
        .to_string()
}

fn path_info(store_path: &str) -> Result<Output> {
    let raw = nix(&["path-info", "--json", "--json-format", "1", store_path])?;
    let infos: BTreeMap<String, PathInfo> =
        serde_json::from_str(&raw).context("parsing nix path-info JSON")?;
    let (path, info) = infos
        .into_iter()
        .next()
        .context("nix path-info returned no paths")?;
    Ok(Output {
        store_path: path,
        nar_hash: info.nar_hash,
        drv_path: info.deriver,
    })
}

fn resolved_path(store: &Store, installable: &str) -> Result<String> {
    if installable.starts_with("/nix/store/") {
        return Ok(installable.to_owned());
    }
    if let Ok(reference) = Ref::parse(installable) {
        if let Some(metadata) = store.load(&reference)? {
            let system = current_system()?;
            return metadata
                .entry
                .outputs
                .get(&system)
                .map(|output| output.store_path.clone())
                .with_context(|| format!("alias `{installable}` has no output for {system}"));
        }
    }
    build_installable(installable)
}

pub fn tag(installable: &str, target: &str, upstream: Option<String>) -> Result<()> {
    let store = Store::open()?;
    let reference = Ref::parse(target)?;
    if reference.root_url.is_some() && upstream.is_none() {
        bail!(
            "qualified names denote remote state; tags are bare. To publish, tag on the box that serves (see docs/design.md \"The org workflow\")."
        );
    }
    let path = resolved_path(&store, installable)?;
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

pub fn untag(target: &str) -> Result<()> {
    let store = Store::open()?;
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

pub fn history(name: &str) -> Result<Vec<HistoryEntry>> {
    Store::open()?.history(name)
}

pub fn list(prefix: Option<&str>, long: bool) -> Result<String> {
    let store = Store::open()?;
    let system = current_system()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(render_list(store.all()?, prefix, long, &system, now))
}

fn render_list(
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
pub fn inspect_artifact(installable: &str) -> Result<Artifact> {
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
        let store = Store::open()?;
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

fn api_entry(metadata: &TagMetadata, substituters: &[String]) -> Entry {
    let mut entry = metadata.entry.clone();
    entry.substituters = substituters.to_vec();
    entry
}

fn negotiated_response(
    body: Vec<u8>,
    status: StatusCode,
    content_type: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", content_type).expect("valid header"))
        .with_header(Header::from_bytes("Vary", "Accept").expect("valid header"))
}

fn json_response<T: Serialize>(
    value: &T,
    status: StatusCode,
) -> Response<std::io::Cursor<Vec<u8>>> {
    negotiated_response(
        serde_json::to_vec(value).expect("serializing API response"),
        status,
        "application/vnd.cix+json;version=1",
    )
}

fn html_response(body: String, status: StatusCode) -> Response<std::io::Cursor<Vec<u8>>> {
    negotiated_response(body.into_bytes(), status, "text/html; charset=utf-8")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn html_document(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font:16px system-ui,sans-serif;max-width:70rem;margin:2rem auto;padding:0 1rem}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #bbb;padding:.4rem;text-align:left;vertical-align:top}}code{{background:#f3f3f3;padding:.15rem .3rem}}</style></head><body>{body}</body></html>",
        html_escape(title)
    )
}

fn age(created_at: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    created_at
        .parse::<u64>()
        .map(|created| format!("{}s", now.saturating_sub(created)))
        .unwrap_or_else(|_| "unknown".into())
}

fn closure_size_text(store_path: &str) -> Option<String> {
    nix(&["path-info", "-S", store_path])
        .ok()
        .and_then(|output| {
            output
                .split_whitespace()
                .last()
                .filter(|size| size.bytes().all(|byte| byte.is_ascii_digit()))
                .map(ToOwned::to_owned)
        })
}

fn manifest_summary(store_path: &str) -> Option<String> {
    let contents = fs::read(Path::new(store_path).join("cix-manifest.json")).ok()?;
    let spec: serde_json::Value = serde_json::from_slice(&contents).ok()?;
    let services = spec.get("services")?.as_object()?;
    let mut rows = String::new();
    for (service_name, service) in services {
        let service = service.as_object()?;
        let ports = service
            .get("ports")
            .and_then(serde_json::Value::as_object)
            .map(|ports| ports.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let environment = service
            .get("env")
            .and_then(serde_json::Value::as_object)
            .map(|environment| environment.keys().cloned().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(service_name),
            html_escape(&ports),
            html_escape(&environment)
        ));
    }
    Some(format!(
        "<section><h2>Spec summary</h2><table><thead><tr><th>Service</th><th>Ports</th><th>Environment</th></tr></thead><tbody>{rows}</tbody></table></section>"
    ))
}

fn header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|candidate| candidate.field.equiv(name))
        .map(|candidate| candidate.value.as_str())
}

fn request_origin(request: &Request) -> (String, String) {
    let host = header(request, "Host").unwrap_or("localhost").to_owned();
    let scheme = header(request, "X-Forwarded-Proto")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("http")
        .to_owned();
    (host, scheme)
}

fn wants_json(request: &Request, query: Option<&str>) -> bool {
    let format = query.and_then(|query| {
        query.split('&').find_map(|parameter| {
            parameter
                .split_once('=')
                .and_then(|(key, value)| (key == "format").then_some(value))
        })
    });
    match format {
        Some("json") => true,
        Some("html") => false,
        _ => header(request, "Accept").is_some_and(|accept| {
            accept.split(',').any(|media_range| {
                let media_type = media_range.split(';').next().unwrap_or_default().trim();
                media_type.eq_ignore_ascii_case("application/vnd.cix+json")
                    || media_type.eq_ignore_ascii_case("application/json")
            })
        }),
    }
}

fn bare_tags(tags: Vec<TagMetadata>) -> Vec<(Ref, TagMetadata)> {
    tags.into_iter()
        .filter_map(|metadata| {
            Ref::parse(&metadata.reference)
                .ok()
                .map(|reference| (reference, metadata))
        })
        .filter(|(reference, _)| reference.root_url.is_none())
        .collect()
}

fn names_page(tags: &[(Ref, TagMetadata)]) -> String {
    let names = tags
        .iter()
        .map(|(reference, _)| reference.name.clone())
        .collect::<BTreeSet<_>>();
    let links = names
        .iter()
        .map(|name| {
            format!(
                "<li><a href=\"/{}\">{}</a></li>",
                html_escape(name),
                html_escape(name)
            )
        })
        .collect::<String>();
    html_document("cix index", format!("<h1>cix index</h1><ul>{links}</ul>"))
}

fn name_page(host: &str, name: &str, tags: &[(Ref, TagMetadata)]) -> String {
    let system = current_system().ok();
    let mut rows = String::new();
    let mut summary = None;
    for (reference, metadata) in tags {
        let output = system
            .as_deref()
            .and_then(|system| metadata.entry.outputs.get(system));
        if summary.is_none() {
            summary = output.and_then(|output| manifest_summary(&output.store_path));
        }
        let systems = metadata
            .entry
            .outputs
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let store_path = output
            .map(|output| output.store_path.as_str())
            .unwrap_or("-");
        let nar_hash = output.map(|output| output.nar_hash.as_str()).unwrap_or("-");
        let size = output
            .and_then(|output| closure_size_text(&output.store_path))
            .unwrap_or_else(|| "unknown".into());
        let permalink = format!("/{}:{}", reference.name, reference.tag);
        rows.push_str(&format!(
            "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&permalink),
            html_escape(&reference.tag),
            html_escape(&systems),
            html_escape(store_path),
            html_escape(nar_hash),
            html_escape(&size),
            html_escape(&age(&metadata.entry.created_at)),
        ));
    }
    let pull_snippets = tags
        .iter()
        .map(|(reference, _)| {
            format!(
                "<p><code>cix pull {}/{}</code></p>",
                html_escape(host),
                html_escape(&format!("{}:{}", name, reference.tag))
            )
        })
        .collect::<String>();
    html_document(
        &format!("{host}/{name}"),
        format!(
            "<h1>{}/{}</h1><table><thead><tr><th>Tag</th><th>Systems</th><th>Store path</th><th>narHash</th><th>Closure size</th><th>Age</th></tr></thead><tbody>{rows}</tbody></table>{pull_snippets}{}",
            html_escape(host),
            html_escape(name),
            summary.unwrap_or_default(),
        ),
    )
}

fn detail_page(host: &str, reference: &Ref, metadata: &TagMetadata) -> String {
    let mut rows = String::new();
    for (system, output) in &metadata.entry.outputs {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(system),
            html_escape(&output.store_path),
            html_escape(&output.nar_hash),
            html_escape(output.drv_path.as_deref().unwrap_or("-")),
        ));
    }
    let full_ref = format!("{host}/{}:{}", reference.name, reference.tag);
    html_document(
        &full_ref,
        format!(
            "<h1>{}</h1><table><thead><tr><th>System</th><th>Store path</th><th>narHash</th><th>drvPath</th></tr></thead><tbody>{rows}</tbody></table><p>createdAt: {}</p><p><code>cix pull {}</code></p><p><code>cix run {}</code></p>",
            html_escape(&full_ref),
            html_escape(&metadata.entry.created_at),
            html_escape(&full_ref),
            html_escape(&full_ref),
        ),
    )
}

fn not_found_html() -> String {
    html_document("Not found", "<h1>Not found</h1>".into())
}

fn copy_to_cache(cache: &Path, output: &Output, sign_key: Option<&str>) -> Result<()> {
    if let Some(key) = sign_key {
        nix(&["store", "sign", "--key-file", key, &output.store_path])?;
    }
    let cache_url = format!("file://{}", cache.display());
    nix(&["copy", "--to", &cache_url, &output.store_path])?;
    Ok(())
}

fn sync_cache(store: &Store, cache: &Path, sign_key: Option<&str>) -> Result<()> {
    if cache.exists() {
        fs::remove_dir_all(cache)
            .with_context(|| format!("rebuilding cix binary cache {}", cache.display()))?;
    }
    fs::create_dir_all(cache)?;
    for metadata in store.all()? {
        let reference = Ref::parse(&metadata.reference)?;
        if reference.root_url.is_none() {
            for output in metadata.entry.outputs.values() {
                copy_to_cache(cache, output, sign_key)?;
            }
        }
    }
    Ok(())
}

/// Serves indefinitely. The request loop reloads sidecars so a long-running
/// server notices tags created after it was started.
pub fn serve(
    listen: &str,
    substituters: Vec<String>,
    with_store: bool,
    sign_key: Option<&str>,
) -> Result<()> {
    let store = Store::open()?;
    let cache = store.root().join("store");
    if with_store {
        sync_cache(&store, &cache, sign_key)?;
    }
    let server = Server::http(listen).map_err(|error| anyhow!(error))?;
    eprintln!("cix index listening on {listen}");
    for request in server.incoming_requests() {
        if request.method() != &Method::Get {
            request.respond(Response::empty(StatusCode(405)))?;
            continue;
        }
        let (url, query) = request.url().split_once('?').unwrap_or((request.url(), ""));
        if with_store && url.starts_with("/store/") {
            let relative = &url["/store/".len()..];
            if relative.contains("..") || relative.is_empty() {
                request.respond(Response::empty(StatusCode(404)))?;
                continue;
            }
            let file = cache.join(relative);
            match fs::File::open(&file) {
                Ok(mut handle) => {
                    let mut bytes = Vec::new();
                    handle.read_to_end(&mut bytes)?;
                    request.respond(Response::from_data(bytes))?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    request.respond(Response::empty(StatusCode(404)))?;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("reading cache file {}", file.display()))
                }
            }
            continue;
        }

        let tags = store.all()?;
        if with_store {
            sync_cache(&store, &cache, sign_key)?;
        }
        let tags = bare_tags(tags);
        let (host, scheme) = request_origin(&request);
        let mut advertised = substituters.clone();
        if with_store {
            advertised.push(format!("{scheme}://{host}/store"));
        }
        let json = wants_json(&request, (!query.is_empty()).then_some(query));
        let response = match url {
            "/" => {
                let names = tags
                    .iter()
                    .map(|(reference, _)| reference.name.clone())
                    .collect::<BTreeSet<_>>();
                if json {
                    json_response(&serde_json::json!({"names": names}), StatusCode(200))
                } else {
                    html_response(names_page(&tags), StatusCode(200))
                }
            }
            path => {
                let input = path.strip_prefix('/').unwrap_or_default();
                let parsed = Ref::parse(input)
                    .ok()
                    .filter(|reference| reference.root_url.is_none());
                let has_tag = input
                    .rsplit_once('/')
                    .map_or(input, |(_, last)| last)
                    .contains(':');
                match (parsed, has_tag) {
                    (Some(reference), true) => {
                        match tags.iter().find(|(candidate, _)| candidate == &reference) {
                            Some((_, metadata)) if json => {
                                json_response(&api_entry(metadata, &advertised), StatusCode(200))
                            }
                            Some((_, metadata)) => html_response(
                                detail_page(&host, &reference, metadata),
                                StatusCode(200),
                            ),
                            None if json => json_response(
                                &serde_json::json!({"error": "unknown tag"}),
                                StatusCode(404),
                            ),
                            None => html_response(not_found_html(), StatusCode(404)),
                        }
                    }
                    (Some(reference), false) => {
                        let matching = tags
                            .iter()
                            .filter(|(candidate, _)| candidate.name == reference.name)
                            .cloned()
                            .collect::<Vec<_>>();
                        if matching.is_empty() {
                            if json {
                                json_response(
                                    &serde_json::json!({"error": "unknown name"}),
                                    StatusCode(404),
                                )
                            } else {
                                html_response(not_found_html(), StatusCode(404))
                            }
                        } else if json {
                            let entries = matching
                                .iter()
                                .map(|(reference, metadata)| {
                                    (reference.tag.clone(), api_entry(metadata, &advertised))
                                })
                                .collect::<BTreeMap<_, _>>();
                            json_response(&serde_json::json!({"tags": entries}), StatusCode(200))
                        } else {
                            html_response(
                                name_page(&host, &reference.name, &matching),
                                StatusCode(200),
                            )
                        }
                    }
                    _ if json => {
                        json_response(&serde_json::json!({"error": "not found"}), StatusCode(404))
                    }
                    _ => html_response(not_found_html(), StatusCode(404)),
                }
            }
        };
        request.respond(response)?;
    }
    Ok(())
}

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

fn resolve_remote(reference: &Ref) -> Result<Entry> {
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

fn fetch_output(reference: &Ref, entry: &Entry, output: &Output) -> Result<()> {
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
pub fn resolve(reference: &str) -> Result<Output> {
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
    let store = Store::open()?;
    store
        .load(&reference)?
        .with_context(|| format!("local tag `{}` does not exist", reference.display()))?
        .entry
        .outputs
        .get(&system)
        .cloned()
        .with_context(|| {
            format!(
                "local tag `{}` has no output for {system}",
                reference.display()
            )
        })
}

fn pull_one(remote: &Ref, local: &Ref) -> Result<bool> {
    let entry = resolve_remote(remote)?;
    let system = current_system()?;
    let output = entry
        .outputs
        .get(&system)
        .with_context(|| format!("remote `{}` has no output for {system}", remote.display()))?;
    let store = Store::open()?;
    if store
        .load(local)?
        .and_then(|metadata| metadata.entry.outputs.get(&system).cloned())
        .is_some_and(|existing| existing.nar_hash == output.nar_hash)
    {
        return Ok(false);
    }
    fetch_output(remote, &entry, output)?;
    tag(
        &output.store_path,
        &local.display(),
        remote.root_url.clone(),
    )?;
    Ok(true)
}

pub fn pull(reference: Option<&str>, as_ref: Option<&str>) -> Result<usize> {
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
            Ok(usize::from(pull_one(&remote, &local)?))
        }
        None => {
            if as_ref.is_some() {
                bail!("--as requires a remote ref");
            }
            let store = Store::open()?;
            let mut changed = 0;
            for metadata in store.all()? {
                let Some(upstream) = metadata.upstream else {
                    continue;
                };
                let local = Ref::parse(&metadata.reference)?;
                let mut remote = local.clone();
                remote.root_url = Some(upstream);
                changed += usize::from(pull_one(&remote, &local)?);
            }
            Ok(changed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{path_info, render_list, Entry, Output, PointerChanged, Store, TagMetadata};
    use cix_common::Ref;
    use std::path::Path;
    use std::{
        collections::BTreeMap,
        fs,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "cix-index-d45-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn test_store(label: &str) -> Store {
        Store::open_at(temporary_path(label)).unwrap()
    }

    fn output(label: &str) -> Output {
        let source = temporary_path(label);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("payload"), label).unwrap();
        let source_text = source.to_string_lossy().into_owned();
        let store_path = cix_common::nix(&["store", "add-path", &source_text])
            .unwrap()
            .trim()
            .to_owned();
        fs::remove_dir_all(source).unwrap();
        path_info(&store_path).unwrap()
    }

    fn metadata(name: &str, tag: &str, output: Output) -> TagMetadata {
        TagMetadata {
            reference: format!("{name}:{tag}"),
            entry: Entry {
                outputs: BTreeMap::from([("x86_64-linux".into(), output)]),
                substituters: vec![],
                trusted_keys: vec![],
                created_at: "1".into(),
            },
            upstream: None,
        }
    }

    fn remove_store(store: &Store) {
        fs::remove_dir_all(store.root()).unwrap();
    }

    #[test]
    fn encoding_is_safe_and_distinct() {
        let left = Ref::parse("cix.example.com/team/app:v1").unwrap();
        let right = Ref::parse("cix.example.com/team/app:v2").unwrap();
        assert_ne!(Store::encode(&left), Store::encode(&right));
        assert_eq!(
            Store::encode_name(&left.name),
            Store::encode_name(&right.name)
        );
        assert!(Store::encode_name(&left.name)
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
    }

    #[test]
    fn publish_resolve_round_trip_and_cli_golden() {
        let store = test_store("round-trip");
        let reference = Ref::parse("localhost:8420/x:v1").unwrap();
        let artifact = output("round-trip-artifact");
        let published = metadata("x", "v1", artifact.clone());
        store.publish("x", "v1", published.clone()).unwrap();
        assert_eq!(store.load(&reference).unwrap(), Some(published));
        assert_eq!(
            render_list(
                vec![metadata(
                    "x",
                    "v1",
                    Output {
                        store_path: "/nix/store/example".into(),
                        nar_hash: "sha256-example".into(),
                        drv_path: None,
                    },
                )],
                None,
                true,
                "x86_64-linux",
                1,
            ),
            "REF   SYSTEMS       PATH                UPSTREAM  AGE\nx:v1  x86_64-linux  /nix/store/example  -         0s "
        );
        remove_store(&store);
    }

    #[test]
    fn cas_conflict_is_distinct() {
        let store = test_store("cas");
        store
            .publish("x", "v1", metadata("x", "v1", output("cas-v1")))
            .unwrap();
        let expected = store.read_pointer("x").unwrap();
        store
            .publish("x", "v2", metadata("x", "v2", output("cas-v2")))
            .unwrap();
        let (_, table) = store.current_table("x").unwrap();
        let candidate = store.add_table(&table).unwrap();
        let error = store
            .cas_pointer("x", expected.as_ref(), &candidate, &table)
            .unwrap_err();
        assert!(error.downcast_ref::<PointerChanged>().is_some());
        remove_store(&store);
    }

    #[test]
    fn identical_tag_tables_have_identical_store_paths() {
        let store = test_store("deterministic-table");
        let (_, table) = store.current_table("x").unwrap();
        assert_eq!(
            store.add_table(&table).unwrap(),
            store.add_table(&table).unwrap()
        );
        remove_store(&store);
    }

    #[test]
    fn multi_tag_publish_is_atomic_for_readers() {
        let store = test_store("many");
        store
            .publish("x", "old", metadata("x", "old", output("many-old")))
            .unwrap();
        let reader_store = store.clone();
        let done = Arc::new(AtomicBool::new(false));
        let reader_done = done.clone();
        let reader = thread::spawn(move || {
            while !reader_done.load(Ordering::Relaxed) {
                let (_, table) = reader_store.current_table("x").unwrap();
                let tags = table.tags.keys().cloned().collect::<Vec<_>>();
                assert!(tags == ["old"] || tags == ["new-a", "new-b", "old"]);
            }
        });
        store
            .publish_many(
                "x",
                vec![
                    ("new-a".into(), metadata("x", "new-a", output("many-a"))),
                    ("new-b".into(), metadata("x", "new-b", output("many-b"))),
                ],
            )
            .unwrap();
        done.store(true, Ordering::Relaxed);
        reader.join().unwrap();
        assert_eq!(
            store
                .current_table("x")
                .unwrap()
                .1
                .tags
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["new-a", "new-b", "old"]
        );
        remove_store(&store);
    }

    #[test]
    fn yank_is_advisory_and_roots_follow_the_current_table() {
        let store = test_store("yank");
        let first = output("yank-first");
        let second = output("yank-second");
        store
            .publish_many(
                "x",
                vec![
                    ("v1".into(), metadata("x", "v1", first.clone())),
                    ("v2".into(), metadata("x", "v2", second.clone())),
                ],
            )
            .unwrap();
        let roots = store.name_roots_dir("x");
        assert_eq!(fs::read_dir(roots.join("paths")).unwrap().count(), 2);
        assert!(store.yank("x", "v1").unwrap());
        assert!(store.load(&Ref::parse("x:v1").unwrap()).unwrap().is_none());
        assert!(Path::new(&first.store_path).join("payload").is_file());
        assert_eq!(fs::read_dir(roots.join("paths")).unwrap().count(), 1);
        assert!(fs::read_link(roots.join("table")).unwrap().is_dir());
        remove_store(&store);
    }

    #[test]
    fn history_walks_available_parent_tables() {
        let store = test_store("history");
        store
            .publish("x", "v1", metadata("x", "v1", output("history-v1")))
            .unwrap();
        store
            .publish("x", "v2", metadata("x", "v2", output("history-v2")))
            .unwrap();
        let history = store.history("x").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].tags, ["v1", "v2"]);
        assert_eq!(history[1].tags, ["v1"]);
        remove_store(&store);
    }

    #[test]
    fn migrates_a_legacy_sidecar_fixture_once() {
        let root = temporary_path("migration");
        let legacy = root.join("tags");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(root.join("roots")).unwrap();
        let artifact = output("migration-artifact");
        let reference = Ref::parse("x:v1").unwrap();
        let legacy_metadata = metadata("x", "v1", artifact);
        let encoded = Store::encode(&reference);
        fs::write(
            legacy.join(format!("{encoded}.json")),
            serde_json::to_vec_pretty(&legacy_metadata).unwrap(),
        )
        .unwrap();
        let legacy_root = root.join("roots").join(&encoded);
        let legacy_root_text = legacy_root.to_string_lossy().into_owned();
        cix_common::nix(&[
            "build",
            &legacy_metadata.entry.outputs["x86_64-linux"].store_path,
            "--out-link",
            &legacy_root_text,
        ])
        .unwrap();

        let store = Store::open_at(root.clone()).unwrap();
        assert_eq!(store.load(&reference).unwrap(), Some(legacy_metadata));
        assert!(root
            .join("meta.legacy")
            .join(format!("{encoded}.json"))
            .is_file());
        assert!(!root.join("tags").exists());
        assert!(!legacy_root.exists());
        drop(store);
        let reopened = Store::open_at(root.clone()).unwrap();
        assert!(reopened.load(&reference).unwrap().is_some());
        remove_store(&reopened);
    }
}
