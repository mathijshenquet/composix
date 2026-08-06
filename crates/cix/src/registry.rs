use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use cix_build::{ArtifactPin, ArtifactResolver};

pub(crate) struct IndexRegistry {
    store: cix_index::Store,
}

impl IndexRegistry {
    pub(crate) fn open(state_directory: PathBuf) -> Result<Self> {
        Ok(Self {
            store: cix_index::Store::open(state_directory)?,
        })
    }
}

impl ArtifactResolver for IndexRegistry {
    fn resolve_artifact(&self, reference: &str) -> Result<ArtifactPin> {
        let output = cix_index::resolve_with(&self.store, reference).with_context(|| {
            format!("resolving cix-item FROM ref {reference:?}; pull it or tag it first")
        })?;
        Ok(ArtifactPin {
            store_path: output.store_path,
            nar_hash: output.nar_hash,
        })
    }
}

impl cix_cixfile::ArtifactRegistry for IndexRegistry {
    fn tag_artifact(&self, store_path: &str, reference: &str) -> Result<()> {
        cix_index::tag(&self.store, store_path, reference, None)
    }
}

pub(crate) struct RunResolver {
    state_directory: PathBuf,
}

impl RunResolver {
    pub(crate) fn new(state_directory: PathBuf) -> Self {
        Self { state_directory }
    }
}

impl cix_run::InstallableResolver for RunResolver {
    fn resolve_installable(&self, installable: &str) -> Result<PathBuf> {
        if installable.is_empty() {
            bail!("installable must not be empty");
        }
        let direct_path = PathBuf::from(installable);
        if direct_path.starts_with("/nix/store/") && direct_path.exists() {
            return Ok(direct_path);
        }

        match cix_common::Ref::parse(installable) {
            Ok(reference) => match cix_index::resolve_with(
                &cix_index::Store::open(self.state_directory.clone())?,
                installable,
            ) {
                Ok(output) => return Ok(PathBuf::from(output.store_path)),
                Err(error) if reference.root_url.is_some() => {
                    return Err(error).with_context(|| {
                        format!("failed to resolve qualified cix ref {installable:?}")
                    });
                }
                Err(_) => {}
            },
            Err(error) if cix_common::Ref::looks_like_untagged_ref(installable) => {
                return Err(error);
            }
            Err(_) => {}
        }

        let output = nix_build(installable)?;
        if !output.status.success() {
            bail!(
                "failed to resolve installable {installable:?}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let paths =
            String::from_utf8(output.stdout).context("nix emitted a non-UTF-8 store path")?;
        let paths = paths
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if paths.len() != 1 {
            bail!(
                "installable {installable:?} resolved to {} outputs; cix run needs exactly one",
                paths.len()
            );
        }
        let path = PathBuf::from(paths[0]);
        if !path.starts_with("/nix/store/") {
            bail!(
                "installable {installable:?} resolved outside the Nix store: {}",
                path.display()
            );
        }
        Ok(path)
    }
}

fn nix_build(installable: &str) -> Result<Output> {
    nix_command(&["build", "--no-link", "--print-out-paths", "--", installable])
        .with_context(|| format!("failed to invoke nix for installable {installable:?}"))
}

fn nix_command(args: &[&str]) -> Result<Output> {
    let invoke = |program: &Path| Command::new(program).args(args).output();
    match invoke(Path::new("nix")) {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            invoke(Path::new("/nix/var/nix/profiles/default/bin/nix"))
                .context("failed to invoke fallback nix executable")
        }
        Err(error) => Err(error).context("failed to invoke nix"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cix_run::InstallableResolver;

    #[test]
    fn resolves_an_existing_store_path_without_building_it() {
        let store_path = std::fs::read_dir("/nix/store")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_dir())
            .unwrap();
        let resolver = RunResolver::new(PathBuf::from("/"));
        assert_eq!(
            resolver
                .resolve_installable(store_path.to_str().unwrap())
                .unwrap(),
            store_path
        );
    }
}
