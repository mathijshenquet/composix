use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once, OnceLock};
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
const STALE_SWEEP_AGE: Duration = Duration::from_secs(6 * 60 * 60);
const OWNER_LOCK_DIRECTORY: &str = ".cix-scratch-locks";

fn is_stale(age: Duration) -> bool {
    age >= STALE_SWEEP_AGE
}

// This process-wide flag is set once per CLI build invocation and read by
// scratch owners as they are created; an atomic avoids shared mutable state in
// the signal-adjacent cleanup path.
static KEEP_SCRATCH: AtomicBool = AtomicBool::new(false);
// A signal-listener thread and ScratchDir drops must coordinate the live paths
// so SIGINT/SIGTERM can clean every active tree before restoring the default
// signal action; this narrow registry is the required shared ownership.
static LIVE: Mutex<Vec<LiveScratch>> = Mutex::new(Vec::new());
// Signal handlers are process-global, so this Once prevents duplicate handler
// threads when more than one build setup path requests scratch cleanup.
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
    owner_lock_path: PathBuf,
    // This descriptor holds an advisory lock that lets another cix process
    // distinguish a live old scratch tree from an orphan during startup sweep.
    _owner_lock: fs::File,
}

#[derive(Clone)]
struct LiveScratch {
    path: PathBuf,
    owner_lock_path: PathBuf,
}

impl ScratchDir {
    pub fn new(prefix: &str) -> Result<Self> {
        let temporary = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(root())
            .with_context(|| format!("creating scratch directory with prefix {prefix}"))?;
        let owner_lock_path = owner_lock_path(temporary.path());
        fs::create_dir_all(
            owner_lock_path
                .parent()
                .expect("scratch owner lock always has a parent"),
        )
        .with_context(|| {
            format!(
                "creating scratch owner lock directory for {}",
                temporary.path().display()
            )
        })?;
        let owner_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&owner_lock_path)
            .with_context(|| {
                format!(
                    "creating scratch owner lock in {}",
                    temporary.path().display()
                )
            })?;
        lock_owner(&owner_lock)
            .with_context(|| format!("locking scratch owner in {}", temporary.path().display()))?;
        let path = temporary.keep();
        LIVE.lock()
            .expect("locking scratch registry")
            .push(LiveScratch {
                path: path.clone(),
                owner_lock_path: owner_lock_path.clone(),
            });
        let scratch = Self {
            path,
            owner_lock_path,
            _owner_lock: owner_lock,
        };
        signal_scratch_ready(scratch.path())?;
        Ok(scratch)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn close(mut self) -> Result<()> {
        let path = std::mem::take(&mut self.path);
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        finish(path, std::mem::take(&mut self.owner_lock_path))
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let path = std::mem::take(&mut self.path);
        if path.as_os_str().is_empty() {
            return;
        }
        if let Err(error) = finish(path, self.owner_lock_path.clone()) {
            eprintln!("warning: failed to clean cix scratch: {error:#}");
        }
    }
}

fn root() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/tmp"))
}

fn signal_scratch_ready(path: &Path) -> Result<()> {
    let Some(signal_path) = std::env::var_os("CIX_SCRATCH_READY_FIFO") else {
        return Ok(());
    };
    // A build may allocate many scratch directories, but the test FIFO has one
    // reader; cache the first signal result so later allocations cannot block.
    static READY_SIGNAL: OnceLock<Result<(), String>> = OnceLock::new();
    let result = READY_SIGNAL.get_or_init(|| {
        let signal = || -> Result<()> {
            let mut signal = OpenOptions::new()
                .write(true)
                .open(&signal_path)
                .with_context(|| {
                    format!(
                        "opening scratch readiness signal {}",
                        Path::new(&signal_path).display()
                    )
                })?;
            writeln!(signal, "{}", path.display()).context("writing scratch readiness signal")?;
            Ok(())
        };
        signal().map_err(|error| format!("{error:#}"))
    });
    result
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error}"))
        .map(|_| ())
}

fn finish(path: PathBuf, owner_lock_path: PathBuf) -> Result<()> {
    LIVE.lock()
        .expect("locking scratch registry")
        .retain(|live| live.path != path);
    if KEEP_SCRATCH.load(Ordering::Relaxed) {
        eprintln!("keeping scratch directory {}", path.display());
        remove_owner_lock(&owner_lock_path)?;
        return Ok(());
    }
    remove_tree(&path).with_context(|| format!("removing scratch directory {}", path.display()))?;
    remove_owner_lock(&owner_lock_path)
}

fn cleanup_live() {
    let paths = LIVE.lock().expect("locking scratch registry").clone();
    for scratch in paths {
        if let Err(error) = finish(scratch.path, scratch.owner_lock_path) {
            eprintln!("warning: failed to clean cix scratch after signal: {error:#}");
        }
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    make_writable(path)?;
    fs::remove_dir_all(path)
}

fn lock_owner(lock: &fs::File) -> std::io::Result<()> {
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn owner_lock_path(path: &Path) -> PathBuf {
    path.parent()
        .expect("scratch directory always has a parent")
        .join(OWNER_LOCK_DIRECTORY)
        .join(
            path.file_name()
                .expect("scratch directory always has a final component"),
        )
}

fn owner_is_live(path: &Path) -> std::io::Result<bool> {
    let lock = match OpenOptions::new()
        .read(true)
        .write(true)
        .open(owner_lock_path(path))
    {
        Ok(lock) => lock,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_UN) };
        Ok(false)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Ok(true)
        } else {
            Err(error)
        }
    }
}

fn remove_owner_lock(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("removing scratch owner lock {}", path.display()))
        }
    }
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
        if !is_stale(age) {
            continue;
        }
        let path = entry.path();
        if owner_is_live(&path)
            .with_context(|| format!("checking scratch owner in {}", path.display()))?
        {
            continue;
        }
        remove_tree(&path)
            .with_context(|| format!("sweeping stale scratch directory {}", path.display()))?;
        remove_owner_lock(&owner_lock_path(&path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_window_is_six_hours() {
        assert!(!is_stale(Duration::from_secs(6 * 60 * 60 - 1)));
        assert!(is_stale(Duration::from_secs(6 * 60 * 60)));
    }
}
