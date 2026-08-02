//! Part 1: the composix index — tag, untag, ls, serve, pull.
//! See docs/design.md "Part 1 — index".

use std::{
    collections::BTreeMap,
    env, fs,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use cix_common::{build_installable, current_system, nix, Ref};
use serde::{Deserialize, Serialize};

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
pub(crate) struct TagRecord {
    pub(crate) store_path: String,
    pub(crate) nar_hash: String,
    pub(crate) meta: TagMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TagTable {
    pub(crate) cix_tag_table: u8,
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
    pub(crate) tags: BTreeMap<String, TagRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TablePointer {
    pub(crate) store_path: String,
    pub(crate) nar_hash: String,
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
pub(crate) struct PathInfo {
    pub(crate) nar_hash: String,
    #[serde(default)]
    pub(crate) deriver: Option<String>,
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

    pub(crate) fn open_at(root: PathBuf) -> Result<Self> {
        let store = Self { root };
        fs::create_dir_all(store.roots_dir()).context("creating cix roots directory")?;
        fs::create_dir_all(store.names_dir()).context("creating cix names directory")?;
        fs::create_dir_all(store.tmp_dir()).context("creating cix temporary directory")?;
        if store.root.join("tags").is_dir() {
            bail!(
                "legacy index tags/ state is unsupported in this alpha; remove or regenerate {} with the current cix",
                store.root.display()
            );
        }
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn roots_dir(&self) -> PathBuf {
        self.root.join("roots")
    }

    pub(crate) fn names_dir(&self) -> PathBuf {
        self.root.join("names")
    }

    pub(crate) fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub fn encode(reference: &Ref) -> String {
        URL_SAFE_NO_PAD.encode(reference.display())
    }

    pub fn encode_name(name: &str) -> String {
        URL_SAFE_NO_PAD.encode(name)
    }

    pub(crate) fn pointer_path(&self, name: &str) -> PathBuf {
        self.names_dir().join(Self::encode_name(name))
    }

    pub(crate) fn pointer_lock_path(&self) -> PathBuf {
        self.names_dir().join(".lock")
    }

    pub(crate) fn name_roots_dir(&self, name: &str) -> PathBuf {
        self.roots_dir().join("names").join(Self::encode_name(name))
    }

    pub(crate) fn read_pointer(&self, name: &str) -> Result<Option<TablePointer>> {
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

    pub(crate) fn write_pointer(&self, name: &str, pointer: &TablePointer) -> Result<()> {
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

    pub(crate) fn with_pointer_lock<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
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

    pub(crate) fn read_table(&self, pointer: &TablePointer) -> Result<TagTable> {
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

    pub(crate) fn current_table(&self, name: &str) -> Result<(Option<TablePointer>, TagTable)> {
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

    pub(crate) fn add_table(&self, table: &TagTable) -> Result<TablePointer> {
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

    pub(crate) fn cas_pointer(
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

    pub(crate) fn record_from_metadata(metadata: TagMetadata) -> Result<TagRecord> {
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
}

pub(crate) fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs()
        .to_string()
}

pub(crate) fn path_info(store_path: &str) -> Result<Output> {
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

pub(crate) fn resolved_path(store: &Store, installable: &str) -> Result<String> {
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
#[cfg(test)]
mod tests {
    use super::{path_info, Entry, Output, PointerChanged, Store, TagMetadata};
    use crate::tags::render_list;
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
        let reference = Ref::parse("localhost:8420/family/x:v1").unwrap();
        let artifact = output("round-trip-artifact");
        let published = metadata("family/x", "v1", artifact.clone());
        store.publish("family/x", "v1", published.clone()).unwrap();
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
    fn legacy_sidecar_state_teaches_rebuild() {
        let root = temporary_path("legacy-state");
        fs::create_dir_all(root.join("tags")).unwrap();
        let error = Store::open_at(root.clone()).unwrap_err().to_string();
        assert!(error.contains("legacy index tags/ state is unsupported"));
        fs::remove_dir_all(root).unwrap();
    }
}
