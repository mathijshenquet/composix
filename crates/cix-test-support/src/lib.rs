//! Shared support for integration tests and generated documentation.

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub content: String,
}

impl GeneratedFile {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }
}

pub fn assert_generated_matches(directory: &Path, expected: &[GeneratedFile]) -> Result<()> {
    let mut expected_names = expected
        .iter()
        .map(|file| PathBuf::from(&file.name))
        .collect::<Vec<_>>();
    expected_names.sort();
    let mut actual_names = generated_names(directory)?;
    actual_names.sort();
    if actual_names != expected_names {
        bail!(
            "{} has added, removed, or renamed pages; regenerate it",
            directory.display()
        );
    }
    for file in expected {
        let path = directory.join(&file.name);
        let actual =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        if actual != file.content {
            bail!("{} has drifted; regenerate it", path.display());
        }
    }
    Ok(())
}

pub fn write_generated_atomically(directory: &Path, files: &[GeneratedFile]) -> Result<()> {
    let parent = directory
        .parent()
        .with_context(|| format!("{} has no parent", directory.display()))?;
    let name = directory
        .file_name()
        .with_context(|| format!("{} has no final component", directory.display()))?
        .to_string_lossy();
    let staging = tempfile::Builder::new()
        .prefix(&format!(".{name}.new-"))
        .tempdir_in(parent)
        .with_context(|| format!("creating staging directory beside {}", directory.display()))?;
    for file in files {
        validate_relative_name(Path::new(&file.name))?;
        let path = staging.path().join(&file.name);
        let file_parent = path.parent().expect("generated path has a parent");
        fs::create_dir_all(file_parent)
            .with_context(|| format!("creating {}", file_parent.display()))?;
        let mut output =
            fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
        output
            .write_all(file.content.as_bytes())
            .with_context(|| format!("writing {}", path.display()))?;
        output
            .sync_all()
            .with_context(|| format!("syncing {}", path.display()))?;
    }
    if directory.exists() {
        // Swap complete sibling directories so a failed generation cannot erase tracked pages.
        rename_exchange(staging.path(), directory)?;
        fs::remove_dir_all(staging.path())
            .with_context(|| format!("removing replaced {}", directory.display()))?;
    } else {
        fs::rename(staging.path(), directory)
            .with_context(|| format!("publishing {}", directory.display()))?;
    }
    Ok(())
}

fn generated_names(directory: &Path) -> Result<Vec<PathBuf>> {
    fn visit(base: &Path, directory: &Path, names: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            fs::read_dir(directory).with_context(|| format!("reading {}", directory.display()))?
        {
            let entry = entry.with_context(|| format!("reading {} entry", directory.display()))?;
            let path = entry.path();
            if path.is_dir() {
                visit(base, &path, names)?;
            } else if path.is_file() {
                names.push(
                    path.strip_prefix(base)
                        .expect("generated file remains below its directory")
                        .to_owned(),
                );
            }
        }
        Ok(())
    }

    let mut names = Vec::new();
    visit(directory, directory, &mut names)?;
    Ok(names)
}

fn validate_relative_name(name: &Path) -> Result<()> {
    if name.as_os_str().is_empty()
        || name.is_absolute()
        || name.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!(
            "generated filename must be a non-empty relative path: {}",
            name.display()
        );
    }
    Ok(())
}

fn rename_exchange(left: &Path, right: &Path) -> Result<()> {
    let left = std::ffi::CString::new(left.as_os_str().as_encoded_bytes())
        .context("staging path contains a NUL byte")?;
    let right = std::ffi::CString::new(right.as_os_str().as_encoded_bytes())
        .context("destination path contains a NUL byte")?;
    // Linux renameat2 exchanges the two fully materialized sibling trees atomically.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            left.as_ptr(),
            libc::AT_FDCWD,
            right.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == -1 {
        return Err(std::io::Error::last_os_error())
            .context("atomically replacing generated pages");
    }
    Ok(())
}

pub fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn find_program(name: &str) -> Result<String> {
    let output = Command::new("sh")
        .args(["-c", "command -v \"$1\"", "sh", name])
        .output()
        .with_context(|| format!("failed to find {name}"))?;
    if !output.status.success() {
        bail!("could not find {name}");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn add_to_store(path: &Path) -> Result<PathBuf> {
    let nix = if Command::new("nix").arg("--version").output().is_ok() {
        "nix"
    } else {
        "/nix/var/nix/profiles/default/bin/nix"
    };
    let output = Command::new(nix)
        .args(["store", "add-path"])
        .arg(path)
        .output()
        .context("failed to invoke nix store add-path")?;
    if !output.status.success() {
        bail!(
            "nix store add-path failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(PathBuf::from(
        String::from_utf8(output.stdout)?.trim().to_owned(),
    ))
}

pub fn wait_for(path: &Path, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.is_file() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("timed out waiting for {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_generation_replaces_a_complete_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("pages");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("old"), "old").unwrap();
        let files = vec![GeneratedFile::new("nested/new", "new")];
        write_generated_atomically(&output, &files).unwrap();
        assert_generated_matches(&output, &files).unwrap();
        assert!(!output.join("old").exists());
    }

    #[test]
    fn failed_generation_leaves_the_published_directory_untouched() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("pages");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join("old"), "old").unwrap();
        let files = vec![GeneratedFile::new("../outside", "not published")];
        assert!(write_generated_atomically(&output, &files).is_err());
        assert_eq!(fs::read_to_string(output.join("old")).unwrap(), "old");
    }
}
