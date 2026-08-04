use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub(crate) const SKELETON_FINGERPRINT: &str = "v2:/usr/bin/env->/bin/env;x86_64:/lib64/ld-linux-x86-64.so.2->/lib/cix-loaders/ld-linux-x86-64.so.2,/lib/ld-musl-x86_64.so.1->/lib/cix-loaders/ld-musl-x86_64.so.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LoaderAlias {
    pub(crate) interpreter: &'static str,
    bridge: &'static str,
    provider_path: &'static str,
    pub(crate) import_hint: &'static str,
}

const X86_64_ALIASES: &[LoaderAlias] = &[
    LoaderAlias {
        interpreter: "/lib64/ld-linux-x86-64.so.2",
        bridge: "/lib/cix-loaders/ld-linux-x86-64.so.2",
        provider_path: "lib/ld-linux-x86-64.so.2",
        import_hint: "${pkgs.glibc}",
    },
    LoaderAlias {
        interpreter: "/lib/ld-musl-x86_64.so.1",
        bridge: "/lib/cix-loaders/ld-musl-x86_64.so.1",
        provider_path: "lib/ld-musl-x86_64.so.1",
        import_hint: "${pkgs.musl}",
    },
];

pub(crate) fn loader_aliases() -> &'static [LoaderAlias] {
    match std::env::consts::ARCH {
        "x86_64" => X86_64_ALIASES,
        _ => &[],
    }
}

pub(crate) struct LoaderSurface {
    directory: tempfile::TempDir,
}

impl LoaderSurface {
    pub(crate) fn new(imports: &[String]) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("cix-import-loaders-")
            .tempdir()
            .context("creating IMPORT loader surface")?;
        for alias in loader_aliases() {
            let Some(provider) = imports
                .iter()
                .map(Path::new)
                .map(|package| package.join(alias.provider_path))
                .find(|candidate| candidate.is_file())
            else {
                continue;
            };
            let bridge = directory.path().join(
                Path::new(alias.bridge)
                    .file_name()
                    .expect("loader bridge has a filename"),
            );
            symlink(&provider, &bridge).with_context(|| {
                format!(
                    "linking imported FHS loader {} to {}",
                    provider.display(),
                    bridge.display()
                )
            })?;
        }
        Ok(Self { directory })
    }

    pub(crate) fn mount(&self, process: &mut Command) {
        if loader_aliases().is_empty() {
            return;
        }
        process.args(["--dir", "/lib"]);
        process
            .arg("--ro-bind")
            .arg(self.directory.path())
            .arg("/lib/cix-loaders");
        process.args(["--dir", "/lib64"]);
        for alias in loader_aliases() {
            process.args(["--symlink", alias.bridge, alias.interpreter]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn loader_surface_uses_the_first_import_and_leaves_other_aliases_dangling() {
        if std::env::consts::ARCH != "x86_64" {
            return;
        }
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for package in [&first, &second] {
            fs::create_dir(package.path().join("lib")).unwrap();
            fs::write(package.path().join("lib/ld-linux-x86-64.so.2"), b"loader").unwrap();
        }
        let imports = [
            first.path().to_string_lossy().into_owned(),
            second.path().to_string_lossy().into_owned(),
        ];
        let surface = LoaderSurface::new(&imports).unwrap();
        assert_eq!(
            fs::read_link(surface.directory.path().join("ld-linux-x86-64.so.2")).unwrap(),
            first.path().join("lib/ld-linux-x86-64.so.2")
        );
        assert!(!surface
            .directory
            .path()
            .join("ld-musl-x86_64.so.1")
            .exists());
    }

    #[test]
    fn empty_imports_keep_the_loader_targets_absent() {
        let surface = LoaderSurface::new(&[]).unwrap();
        assert!(fs::read_dir(surface.directory.path())
            .unwrap()
            .next()
            .is_none());
    }
}
