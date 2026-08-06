//! NAR-invariant identities for filesystem objects used as semantic inputs.

use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Hashes a filesystem object as NAR-relevant identity: kind, content,
/// executable bit, and symlink target. Ownership, timestamps, inode, device,
/// and non-executable permission bits are deliberately absent.
pub fn nar_identity(path: &Path) -> Result<String> {
    let mut digest = Sha256::new();
    append_nar_identity(&mut digest, path)?;
    Ok(hex(digest.finalize()))
}

fn append_nar_identity(digest: &mut Sha256, path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading filesystem identity for {}", path.display()))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        digest.update(b"symlink\0");
        append_bytes(digest, fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if file_type.is_file() {
        digest.update(b"file\0");
        digest.update([executable_bit(&metadata)]);
        let mut file = fs::File::open(path)
            .with_context(|| format!("opening filesystem identity input {}", path.display()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .with_context(|| format!("reading filesystem identity input {}", path.display()))?;
        append_bytes(digest, &bytes);
    } else if file_type.is_dir() {
        digest.update(b"directory\0");
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("opening filesystem identity directory {}", path.display()))?
            .collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            append_bytes(digest, entry.file_name().as_encoded_bytes());
            append_nar_identity(digest, &entry.path())?;
        }
    } else {
        anyhow::bail!(
            "unsupported special file in filesystem identity: {}",
            path.display()
        );
    }
    Ok(())
}

/// Returns the executable identity bit used by [`nar_identity`].
pub fn executable_bit(metadata: &fs::Metadata) -> u8 {
    u8::from(metadata.permissions().mode() & 0o111 != 0)
}

fn append_bytes(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    use super::nar_identity;

    #[test]
    fn nar_identity_ignores_non_executable_permissions_and_preserves_executability() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file");
        fs::write(&path, "same bytes").unwrap();

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let ordinary = nar_identity(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let private = nar_identity(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let executable = nar_identity(&path).unwrap();

        assert_eq!(ordinary, private);
        assert_ne!(ordinary, executable);
    }

    #[test]
    fn symlink_identity_does_not_follow_its_target() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("target"), "first").unwrap();
        symlink("target", root.path().join("link")).unwrap();
        let first = nar_identity(&root.path().join("link")).unwrap();
        fs::write(root.path().join("target"), "second").unwrap();
        assert_eq!(first, nar_identity(&root.path().join("link")).unwrap());
    }

    #[test]
    fn directory_identity_ignores_its_permission_bits() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("directory");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("file"), "same bytes").unwrap();

        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let ordinary = nar_identity(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(ordinary, nar_identity(&path).unwrap());
    }
}
