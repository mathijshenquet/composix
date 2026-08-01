//! GC roots are the durable ownership layer beneath mutable tag references.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use cix_common::nix;

use super::refs::*;

impl Store {
    pub(crate) fn replace_root(&self, link: &Path, store_path: &str) -> Result<()> {
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

    pub(crate) fn referenced_paths(table: &TagTable) -> BTreeSet<String> {
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

    pub(crate) fn sync_roots(&self, table: &TagTable, pointer: &TablePointer) -> Result<()> {
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
}
