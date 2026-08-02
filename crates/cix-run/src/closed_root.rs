use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedRootOptions {
    root_directory: PathBuf,
    gc_root_directory: PathBuf,
    identity_override: Option<String>,
    resolver_source: PathBuf,
}

impl ClosedRootOptions {
    pub fn root_directory(&self) -> &Path {
        &self.root_directory
    }

    pub(crate) fn gc_root_directory(&self) -> &Path {
        &self.gc_root_directory
    }

    pub fn with_identity_override(mut self, identity: impl Into<String>) -> Self {
        self.identity_override = Some(identity.into());
        self
    }

    pub(crate) fn identity_override(&self) -> Option<&str> {
        self.identity_override.as_deref()
    }

    pub fn with_resolver_source(mut self, source: impl Into<PathBuf>) -> Self {
        self.resolver_source = source.into();
        self
    }

    pub(crate) fn resolver_source(&self) -> &Path {
        &self.resolver_source
    }
}

pub fn options_for_unit(unit: &str, user: bool) -> Result<ClosedRootOptions> {
    if unit.is_empty()
        || unit.contains('/')
        || !unit
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'@'))
    {
        bail!("closed-root unit name {unit:?} is not a safe systemd unit name");
    }
    let runtime = if user {
        PathBuf::from(
            std::env::var_os("XDG_RUNTIME_DIR")
                .context("XDG_RUNTIME_DIR is not set for the user manager")?,
        )
        .join("cix")
    } else {
        PathBuf::from("/run/cix")
    };
    Ok(ClosedRootOptions {
        root_directory: runtime.join("closed-roots").join(unit),
        gc_root_directory: runtime.join("gcroots"),
        identity_override: None,
        resolver_source: PathBuf::from("/etc/resolv.conf"),
    })
}

pub fn prepare(options: &ClosedRootOptions) -> Result<()> {
    let root = &options.root_directory;
    let etc = root.join("etc");
    let usr_bin = root.join("usr/bin");
    let nss = root.join("nss");
    fs::create_dir_all(&options.gc_root_directory).with_context(|| {
        format!(
            "creating closed-root GC-root directory {}",
            options.gc_root_directory.display()
        )
    })?;
    fs::create_dir_all(&etc)
        .with_context(|| format!("creating closed-root directory {}", etc.display()))?;
    fs::create_dir_all(&usr_bin)
        .with_context(|| format!("creating closed-root directory {}", usr_bin.display()))?;
    fs::create_dir_all(&nss)
        .with_context(|| format!("creating closed-root directory {}", nss.display()))?;
    for name in ["passwd", "group"] {
        let path = nss.join(name);
        if !path.exists() {
            fs::write(&path, [])
                .with_context(|| format!("creating closed-root NSS file {}", path.display()))?;
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).with_context(|| {
            format!("setting closed-root NSS permissions on {}", path.display())
        })?;
    }
    let env = usr_bin.join("env");
    match fs::read_link(&env) {
        Ok(target) if target == Path::new("/bin/env") => {}
        Ok(target) => bail!(
            "closed-root env alias {} points to {}, expected /bin/env",
            env.display(),
            target.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            symlink("/bin/env", &env)
                .with_context(|| format!("creating closed-root env alias {}", env.display()))?;
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting closed-root env alias {}", env.display()))
        }
    }
    Ok(())
}

pub fn remove(options: &ClosedRootOptions) -> Result<()> {
    if !options
        .root_directory
        .parent()
        .is_some_and(|parent| parent.ends_with("cix/closed-roots"))
    {
        bail!(
            "refusing to remove unrecognized closed-root path {}",
            options.root_directory.display()
        );
    }
    match fs::remove_dir_all(&options.root_directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing closed-root directory {}",
                options.root_directory.display()
            )
        }),
    }
}

pub fn write_nss_for_directory(identity: &str, directory: &Path) -> Result<()> {
    validate_identity(identity)?;
    let metadata = fs::metadata(directory).with_context(|| {
        format!(
            "reading closed-root identity from managed directory {}",
            directory.display()
        )
    })?;
    use std::os::unix::fs::MetadataExt;
    let uid = metadata.uid();
    let gid = metadata.gid();
    let (passwd, group) = synthetic_nss(identity, uid, gid);
    write_nss_file(Path::new("/etc/passwd"), &passwd)?;
    write_nss_file(Path::new("/etc/group"), &group)
}

fn write_nss_file(path: &Path, contents: &str) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o644))
        .with_context(|| format!("opening generated NSS file {} for update", path.display()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("opening generated NSS file {}", path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("writing generated NSS file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing generated NSS file {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o444))
        .with_context(|| format!("sealing generated NSS file {}", path.display()))
}

fn validate_identity(identity: &str) -> Result<()> {
    if identity.is_empty()
        || identity.contains(['\n', '\r', ':'])
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        bail!("closed-root identity {identity:?} is not a safe passwd/group name");
    }
    Ok(())
}

fn synthetic_nss(identity: &str, uid: u32, gid: u32) -> (String, String) {
    (
        format!(
            "root:x:0:0:root:/root:/sbin/nologin\n{identity}:x:{uid}:{gid}:cix service:/:/sbin/nologin\nnobody:x:65534:65534:nobody:/:/sbin/nologin\n"
        ),
        format!("root:x:0:\n{identity}:x:{gid}:\nnobody:x:65534:\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_nss_has_exactly_root_unit_and_nobody() {
        let (passwd, group) = synthetic_nss("cix-api", 61234, 61235);
        assert_eq!(
            passwd.lines().collect::<Vec<_>>(),
            [
                "root:x:0:0:root:/root:/sbin/nologin",
                "cix-api:x:61234:61235:cix service:/:/sbin/nologin",
                "nobody:x:65534:65534:nobody:/:/sbin/nologin",
            ]
        );
        assert_eq!(
            group.lines().collect::<Vec<_>>(),
            ["root:x:0:", "cix-api:x:61235:", "nobody:x:65534:"]
        );
    }

    #[test]
    fn unsafe_identity_is_refused() {
        assert!(validate_identity("escape:0:0").is_err());
        assert!(validate_identity("line\nbreak").is_err());
    }

    #[test]
    fn preparation_creates_root_and_gc_root_source() {
        let temporary = tempfile::tempdir().unwrap();
        let options = ClosedRootOptions {
            root_directory: temporary.path().join("closed-roots/cix-test.service"),
            gc_root_directory: temporary.path().join("gcroots"),
            identity_override: None,
            resolver_source: PathBuf::from("/etc/resolv.conf"),
        };

        prepare(&options).unwrap();

        assert!(options.gc_root_directory.is_dir());
        assert!(options.root_directory.join("nss/passwd").is_file());
        assert_eq!(
            fs::read_link(options.root_directory.join("usr/bin/env")).unwrap(),
            Path::new("/bin/env")
        );
    }
}
