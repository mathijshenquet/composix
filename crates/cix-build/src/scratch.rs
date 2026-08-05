use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

const PREFIXES: &[&str] = &[
    "cix-build-cold-",
    "cix-build-view-",
    "cix-fetch-probe-",
    "cix-fetch-work-",
    "cix-import-loaders-",
    "cix-import-union-",
    "cix-read-trace-",
    "cix-step-delta-",
];

// This process-wide flag is set once per CLI build invocation and read by
// scratch owners as they are created; an atomic avoids shared mutable state in
// the signal-adjacent cleanup path.
static KEEP_SCRATCH: AtomicBool = AtomicBool::new(false);
// A signal-listener thread and ScratchDir drops must coordinate the live paths
// so SIGINT/SIGTERM can clean every active tree before restoring the default
// signal action; this narrow registry is the required shared ownership.
static LIVE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static SIGNAL_CLEANUP: Once = Once::new();

pub fn configure(keep_scratch: bool) {
    KEEP_SCRATCH.store(keep_scratch, Ordering::Relaxed);
}

pub fn install_signal_cleanup() {
    SIGNAL_CLEANUP.call_once(|| {
        let mut signals = signal_hook::iterator::Signals::new([
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGHUP,
        ])
        .expect("installing cix scratch signal handlers");
        std::thread::spawn(move || {
            if let Some(signal) = signals.forever().next() {
                cleanup_live();
                unsafe {
                    libc::signal(signal, libc::SIG_DFL);
                    libc::raise(signal);
                }
            }
        });
    });
}

pub struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    pub fn new(prefix: &str) -> Result<Self> {
        let temporary = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(root())
            .with_context(|| format!("creating scratch directory with prefix {prefix}"))?;
        let path = temporary.keep();
        LIVE.lock()
            .expect("locking scratch registry")
            .push(path.clone());
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn close(mut self) -> Result<()> {
        let path = std::mem::take(&mut self.path);
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        finish(path)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let path = std::mem::take(&mut self.path);
        if path.as_os_str().is_empty() {
            return;
        }
        if let Err(error) = finish(path) {
            eprintln!("warning: failed to clean cix scratch: {error:#}");
        }
    }
}

fn root() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/tmp"))
}

fn finish(path: PathBuf) -> Result<()> {
    LIVE.lock()
        .expect("locking scratch registry")
        .retain(|live| live != &path);
    if KEEP_SCRATCH.load(Ordering::Relaxed) {
        eprintln!("keeping scratch directory {}", path.display());
        return Ok(());
    }
    remove_tree(&path).with_context(|| format!("removing scratch directory {}", path.display()))
}

fn cleanup_live() {
    let paths = LIVE.lock().expect("locking scratch registry").clone();
    for path in paths {
        if let Err(error) = finish(path) {
            eprintln!("warning: failed to clean cix scratch after signal: {error:#}");
        }
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    make_writable(path)?;
    fs::remove_dir_all(path)
}

fn make_writable(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable(&entry?.path())?;
        }
    }
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o700);
    fs::set_permissions(path, permissions)
}

pub fn sweep_stale() -> Result<()> {
    let root = root();
    let now = SystemTime::now();
    let uid = unsafe { libc::geteuid() };
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("reading scratch root {}", root.display()))
        }
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("reading scratch root {}", root.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.uid() != uid || !metadata.is_dir() {
            continue;
        }
        let age = now.duration_since(metadata.modified()?).unwrap_or_default();
        if age < Duration::from_secs(24 * 60 * 60) {
            continue;
        }
        let path = entry.path();
        remove_tree(&path)
            .with_context(|| format!("sweeping stale scratch directory {}", path.display()))?;
    }
    Ok(())
}
