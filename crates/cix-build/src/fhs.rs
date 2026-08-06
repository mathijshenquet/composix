use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::trace::FailureTrace;

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

#[derive(Debug, Eq, PartialEq)]
struct ElfInfo {
    interpreter: Option<String>,
    needed: Vec<String>,
}

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

#[derive(Clone, Copy)]
struct Segment {
    kind: u64,
    offset: u64,
    address: u64,
    size: u64,
}

pub(crate) struct LoaderSurface {
    directory: crate::ScratchDir,
}

impl LoaderSurface {
    pub(crate) fn new(imports: &[String]) -> Result<Self> {
        let directory = crate::ScratchDir::new("cix-import-loaders-")
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

pub(crate) fn failure_hint(
    workdir: &Path,
    imports: &[String],
    failure: &FailureTrace,
) -> Option<String> {
    let mut hints = BTreeSet::new();
    for relative in &failure.work_execs {
        let path = if relative == "." {
            workdir.to_owned()
        } else {
            workdir.join(relative)
        };
        let Ok(Some(elf)) = inspect_elf(&path) else {
            continue;
        };
        let Some(interpreter) = elf.interpreter.as_deref() else {
            continue;
        };
        let Some(alias) = loader_aliases()
            .iter()
            .find(|alias| alias.interpreter == interpreter)
        else {
            continue;
        };
        let name = Path::new(relative)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(relative);
        let provider = imported_loader_provider(imports, alias);
        if provider.is_none() && failure.exec_enoent.contains(relative) {
            hints.insert(missing_loader_hint_for_elf(name, &elf, alias));
            continue;
        }
        let Some(provider) = provider else {
            continue;
        };
        let beyond = elf
            .needed
            .iter()
            .filter(|needed| failure.missing_sonames.contains(*needed))
            .filter(|needed| !provider.join("lib").join(needed).exists())
            .cloned()
            .collect::<BTreeSet<_>>();
        if beyond.is_empty() {
            continue;
        }
        hints.insert(format!(
            "hint: {name} uses {interpreter} from imported libc but also needs libraries beyond that libc: {}; the aliases-only FHS surface does not add a /lib search path. IMPORT ${{pkgs.patchelf}} plus the library providers and use the taught patchelf RUN escape; see docs/migrate.md#fhs-linked-native-binaries",
            beyond.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    for interpreter in &failure.missing_loaders {
        let Some(alias) = loader_aliases()
            .iter()
            .find(|alias| alias.interpreter == interpreter)
        else {
            continue;
        };
        if imported_loader_provider(imports, alias).is_none() {
            hints.insert(format!(
                "hint: this RUN tried the FHS loader {interpreter}; IMPORT {}",
                alias.import_hint
            ));
        }
    }
    (!hints.is_empty()).then(|| hints.into_iter().collect::<Vec<_>>().join("\n"))
}

/// Diagnoses an argv target that bubblewrap could not exec even though the
/// target exists in the workspace. That is the ELF loader's ENOENT, not a
/// missing command.
pub(crate) fn argv_enoent_hint(workdir: &Path, imports: &[String], target: &str) -> Option<String> {
    let path = workdir.join(target);
    let Ok(Some(elf)) = inspect_elf(&path) else {
        return None;
    };
    let interpreter = elf.interpreter.as_deref()?;
    let alias = loader_aliases()
        .iter()
        .find(|alias| alias.interpreter == interpreter)?;
    imported_loader_provider(imports, alias).is_none().then(|| {
        let name = Path::new(target)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(target);
        missing_loader_hint_for_elf(name, &elf, alias)
    })
}

fn missing_loader_hint_for_elf(name: &str, elf: &ElfInfo, alias: &LoaderAlias) -> String {
    let needed = if elf.needed.is_empty() {
        String::new()
    } else {
        format!(" and {}", elf.needed.join(", "))
    };
    format!(
        "hint: {name} requires the FHS loader {}{needed}; IMPORT {}",
        alias.interpreter, alias.import_hint
    )
}

fn imported_loader_provider<'a>(imports: &'a [String], alias: &LoaderAlias) -> Option<&'a Path> {
    imports
        .iter()
        .map(Path::new)
        .find(|package| package.join(alias.provider_path).is_file())
}

fn inspect_elf(path: &Path) -> Result<Option<ElfInfo>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading ELF {}", path.display())),
    };
    if bytes.get(..4) != Some(b"\x7fELF") {
        return Ok(None);
    }
    let class = *bytes.get(4).context("ELF is missing its class")?;
    let order = match bytes.get(5) {
        Some(1) => ByteOrder::Little,
        Some(2) => ByteOrder::Big,
        _ => return Ok(None),
    };
    let (phoff, phentsize, phnum, offset_at, address_at, size_at) = match class {
        1 => (
            integer(&bytes, 28, 4, order)?,
            integer(&bytes, 42, 2, order)?,
            integer(&bytes, 44, 2, order)?,
            4,
            8,
            16,
        ),
        2 => (
            integer(&bytes, 32, 8, order)?,
            integer(&bytes, 54, 2, order)?,
            integer(&bytes, 56, 2, order)?,
            8,
            16,
            32,
        ),
        _ => return Ok(None),
    };
    let word = if class == 1 { 4 } else { 8 };
    let mut segments = Vec::new();
    for index in 0..phnum {
        let start = phoff
            .checked_add(
                index
                    .checked_mul(phentsize)
                    .context("ELF program-header overflow")?,
            )
            .context("ELF program-header overflow")?;
        let start = usize::try_from(start).context("ELF program header is too large")?;
        segments.push(Segment {
            kind: integer(&bytes, start, 4, order)?,
            offset: integer(&bytes, start + offset_at, word, order)?,
            address: integer(&bytes, start + address_at, word, order)?,
            size: integer(&bytes, start + size_at, word, order)?,
        });
    }
    let interpreter = segments
        .iter()
        .find(|segment| segment.kind == 3)
        .and_then(|segment| file_string(&bytes, segment.offset, segment.size).ok());
    let Some(dynamic) = segments.iter().find(|segment| segment.kind == 2) else {
        return Ok(Some(ElfInfo {
            interpreter,
            needed: Vec::new(),
        }));
    };
    let entry_size = word * 2;
    let entries = dynamic.size / u64::try_from(entry_size).expect("ELF word size fits u64");
    let mut string_address = None;
    let mut string_size = None;
    let mut needed_offsets = Vec::new();
    for index in 0..entries {
        let start = dynamic
            .offset
            .checked_add(index * u64::try_from(entry_size).expect("ELF word size fits u64"))
            .context("ELF dynamic-table overflow")?;
        let start = usize::try_from(start).context("ELF dynamic entry is too large")?;
        let tag = integer(&bytes, start, word, order)?;
        let value = integer(&bytes, start + word, word, order)?;
        match tag {
            0 => break,
            1 => needed_offsets.push(value),
            5 => string_address = Some(value),
            10 => string_size = Some(value),
            _ => {}
        }
    }
    let needed = match string_address {
        Some(address) => {
            let offset =
                virtual_file_offset(&segments, address).context("ELF DT_STRTAB is unmapped")?;
            let available = u64::try_from(bytes.len())
                .expect("usize fits u64")
                .saturating_sub(offset);
            let size = string_size.unwrap_or(available).min(available);
            needed_offsets
                .into_iter()
                .map(|needed| file_string(&bytes, offset + needed, size.saturating_sub(needed)))
                .collect::<Result<Vec<_>>>()?
        }
        None => Vec::new(),
    };
    Ok(Some(ElfInfo {
        interpreter,
        needed,
    }))
}

fn integer(bytes: &[u8], offset: usize, width: usize, order: ByteOrder) -> Result<u64> {
    let value = bytes
        .get(offset..offset.checked_add(width).context("ELF integer overflow")?)
        .context("ELF integer is out of bounds")?;
    Ok(match order {
        ByteOrder::Little => value.iter().enumerate().fold(0, |result, (shift, byte)| {
            result | u64::from(*byte) << (shift * 8)
        }),
        ByteOrder::Big => value
            .iter()
            .fold(0, |result, byte| result << 8 | u64::from(*byte)),
    })
}

fn virtual_file_offset(segments: &[Segment], address: u64) -> Option<u64> {
    segments.iter().find_map(|segment| {
        (segment.kind == 1
            && address >= segment.address
            && address < segment.address.checked_add(segment.size)?)
        .then(|| segment.offset + (address - segment.address))
    })
}

fn file_string(bytes: &[u8], offset: u64, size: u64) -> Result<String> {
    let start = usize::try_from(offset).context("ELF string offset is too large")?;
    let size = usize::try_from(size).context("ELF string size is too large")?;
    let value = bytes
        .get(start..start.checked_add(size).context("ELF string overflow")?)
        .context("ELF string is out of bounds")?;
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    Ok(String::from_utf8_lossy(&value[..end]).into_owned())
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

    #[test]
    fn direct_loader_lookup_hint_respects_imported_providers() {
        if std::env::consts::ARCH != "x86_64" {
            return;
        }
        let failure = FailureTrace {
            missing_loaders: BTreeSet::from(["/lib64/ld-linux-x86-64.so.2".into()]),
            ..FailureTrace::default()
        };
        let hint = failure_hint(Path::new("/work"), &[], &failure).unwrap();
        assert!(hint.contains("IMPORT ${pkgs.glibc}"), "{hint}");

        let glibc = tempfile::tempdir().unwrap();
        fs::create_dir(glibc.path().join("lib")).unwrap();
        fs::write(glibc.path().join("lib/ld-linux-x86-64.so.2"), b"loader").unwrap();
        assert_eq!(
            failure_hint(
                Path::new("/work"),
                &[glibc.path().to_string_lossy().into_owned()],
                &failure
            ),
            None
        );
    }
}
