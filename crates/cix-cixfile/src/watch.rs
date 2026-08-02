use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Once,
    },
    time::Duration,
};

use anyhow::{bail, Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{build, BuildOptions, BuiltItem};

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(300);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static INTERRUPT_HANDLER: Once = Once::new();

pub fn watch(path: &Path) -> Result<()> {
    install_interrupt_handler();
    INTERRUPTED.store(false, Ordering::SeqCst);
    let context = WatchContext::new(path)?;
    watch_loop(context)
}

fn install_interrupt_handler() {
    INTERRUPT_HANDLER.call_once(|| {
        ctrlc::set_handler(|| {
            INTERRUPTED.store(true, Ordering::SeqCst);
        })
        .expect("installing cix watch Ctrl-C handler");
    });
}

fn watch_loop(context: WatchContext) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let _watcher = start_watcher(&context.root, sender)?;
    eprintln!("watching {}", context.root.display());
    let debounce = debounce();

    while !INTERRUPTED.load(Ordering::SeqCst) {
        let mut changed = receive_changes(&receiver, &context, Duration::from_millis(50));
        if changed.is_empty() {
            continue;
        }
        loop {
            let more = receive_changes(&receiver, &context, debounce);
            if more.is_empty() {
                break;
            }
            changed.extend(more);
        }
        if INTERRUPTED.load(Ordering::SeqCst) {
            break;
        }
        if let Err(error) = run_round(&context, &changed) {
            if INTERRUPTED.load(Ordering::SeqCst) {
                break;
            }
            eprintln!("watch round failed: {error:#}");
        }
    }
    Ok(())
}

fn debounce() -> Duration {
    env::var("CIX_WATCH_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_DEBOUNCE)
}

fn receive_changes(
    receiver: &Receiver<notify::Result<Event>>,
    context: &WatchContext,
    timeout: Duration,
) -> BTreeSet<PathBuf> {
    match receiver.recv_timeout(timeout) {
        Ok(Ok(event)) => {
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return BTreeSet::new();
            }
            let ignores = IgnoreSet::new(&context.root);
            event
                .paths
                .into_iter()
                .filter(|path| !ignores.ignores(path))
                .collect()
        }
        Ok(Err(error)) => {
            eprintln!("watch event error: {error}");
            BTreeSet::new()
        }
        Err(mpsc::RecvTimeoutError::Timeout) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            BTreeSet::new()
        }
    }
}

fn start_watcher(root: &Path, sender: Sender<notify::Result<Event>>) -> Result<Box<dyn Watcher>> {
    let native = RecommendedWatcher::new(
        {
            let sender = sender.clone();
            move |event| {
                let _ = sender.send(event);
            }
        },
        Config::default(),
    );
    match native {
        Ok(mut watcher) => match watcher.watch(root, RecursiveMode::Recursive) {
            Ok(()) => Ok(Box::new(watcher)),
            Err(error) => start_polling(root, sender, error),
        },
        Err(error) => start_polling(root, sender, error),
    }
}

fn start_polling(
    root: &Path,
    sender: Sender<notify::Result<Event>>,
    native_error: notify::Error,
) -> Result<Box<dyn Watcher>> {
    eprintln!("notify initialization failed ({native_error}); falling back to polling");
    let mut watcher = PollWatcher::new(
        move |event| {
            let _ = sender.send(event);
        },
        Config::default().with_poll_interval(Duration::from_millis(200)),
    )
    .context("initializing polling watcher")?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .context("watching Cixfile context with polling")?;
    Ok(Box::new(watcher))
}

struct WatchContext {
    root: PathBuf,
    compose: Option<PathBuf>,
}

impl WatchContext {
    fn new(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .with_context(|| format!("resolving watch path {}", path.display()))?;
        let root = if path.is_file() {
            if path.file_name().is_some_and(|name| name == "Cixfile") {
                path.parent()
                    .context("Cixfile has no parent directory")?
                    .to_owned()
            } else {
                bail!(
                    "watch path must be a Cixfile or directory, got {}",
                    path.display()
                );
            }
        } else {
            path
        };
        let compose = root
            .join("compose.json")
            .is_file()
            .then(|| root.join("compose.json"));
        if compose.is_none() && !root.join("Cixfile").is_file() {
            bail!(
                "{} contains neither Cixfile nor compose.json",
                root.display()
            );
        }
        Ok(Self { root, compose })
    }
}

fn run_round(context: &WatchContext, paths: &BTreeSet<PathBuf>) -> Result<()> {
    match &context.compose {
        Some(compose) => run_compose_round(context, compose, paths),
        None => run_bare_round(&context.root),
    }
}

fn run_bare_round(directory: &Path) -> Result<()> {
    for item in build(&BuildOptions {
        directory: directory.to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
    })? {
        println!("{}", item.store_path);
    }
    Ok(())
}

fn run_compose_round(
    context: &WatchContext,
    compose_path: &Path,
    paths: &BTreeSet<PathBuf>,
) -> Result<()> {
    let compose = cix_compose::Compose::load(compose_path)?;
    let cixfiles = paths
        .iter()
        .filter_map(|path| cixfile_for(path, &context.root))
        .collect::<BTreeSet<_>>();
    let compose_changed = paths.iter().any(|path| path == compose_path);
    let mut changed_services = BTreeSet::new();

    for cixfile in cixfiles {
        let directory = cixfile.parent().expect("Cixfile has a parent");
        let outputs = build(&BuildOptions {
            directory: directory.to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
            allow_secret: false,
        })?;
        for (service, item) in map_outputs(&compose, &context.root, directory, outputs)? {
            cix_index::tag(&item.store_path, &compose.services[&service].item, None).with_context(
                || {
                    format!(
                        "updating compose service {service:?} from {}",
                        directory.display()
                    )
                },
            )?;
            changed_services.insert(service);
        }
    }

    if !changed_services.is_empty() {
        cix_compose::up(
            compose_path,
            cix_compose::UpdateRequest::Services(changed_services),
        )?;
    } else if compose_changed {
        cix_compose::up(compose_path, cix_compose::UpdateRequest::None)?;
    }
    Ok(())
}

fn cixfile_for(path: &Path, root: &Path) -> Option<PathBuf> {
    let mut directory = if path.is_dir() {
        path.to_owned()
    } else {
        path.parent()?.to_owned()
    };
    loop {
        let cixfile = directory.join("Cixfile");
        if cixfile.is_file() {
            return Some(cixfile);
        }
        if directory == root || !directory.pop() {
            return None;
        }
    }
}

fn map_outputs(
    compose: &cix_compose::Compose,
    root: &Path,
    directory: &Path,
    outputs: Vec<BuiltItem>,
) -> Result<BTreeMap<String, BuiltItem>> {
    let mut mapped = BTreeMap::new();
    for output in &outputs {
        if compose.services.contains_key(&output.name) {
            mapped.insert(output.name.clone(), output.clone());
        }
    }
    if outputs.len() == 1 {
        let directory_service = directory
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.components().next())
            .and_then(|component| component.as_os_str().to_str());
        if let Some(service) =
            directory_service.filter(|service| compose.services.contains_key(*service))
        {
            mapped.insert(
                service.to_owned(),
                outputs.into_iter().next().expect("one output"),
            );
        }
    }
    if mapped.is_empty() {
        bail!(
            "{} rebuilt no compose member: use a Cixfile member named after a compose service, or put a single-member Cixfile in that service's directory",
            directory.display()
        );
    }
    for service in mapped.keys() {
        let reference = cix_common::Ref::parse(&compose.services[service].item)?;
        if reference.root_url.is_some() {
            bail!("compose service {service:?} has a remote item; cix watch can only retag local items");
        }
    }
    Ok(mapped)
}

struct IgnoreSet {
    root: PathBuf,
    special: Vec<PathBuf>,
    gitignore: Gitignore,
}

impl IgnoreSet {
    fn new(root: &Path) -> Self {
        let mut builder = GitignoreBuilder::new(root);
        for path in gitignore_files(root) {
            let _ = builder.add(path);
        }
        Self {
            root: root.to_owned(),
            special: vec![
                root.join(".git"),
                root.join("target"),
                root.join("Cixfile.lock"),
                root.join("cix.lock"),
                workspace_base(),
            ],
            gitignore: builder.build().unwrap_or_else(|error| {
                eprintln!("warning: ignoring invalid .gitignore while watching: {error}");
                GitignoreBuilder::new(root)
                    .build()
                    .expect("empty gitignore")
            }),
        }
    }

    fn ignores(&self, path: &Path) -> bool {
        let path = absolute_path(path, &self.root);
        if path == self.root {
            return true;
        }
        if path.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git") | Some("target")
            )
        }) || path.file_name().is_some_and(is_cix_lock_output)
        {
            return true;
        }
        if self.special.iter().any(|ignored| path.starts_with(ignored)) {
            return true;
        }
        self.gitignore
            .matched_path_or_any_parents(&path, path.is_dir())
            .is_ignore()
    }
}

fn is_cix_lock_output(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        name == "Cixfile.lock"
            || name.starts_with("Cixfile.lock.")
            || name == "cix.lock"
            || name.starts_with("cix.lock.")
    })
}

fn absolute_path(path: &Path, root: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn workspace_base() -> PathBuf {
    let path = if let Some(path) = env::var_os("CIX_BUILD_WORKSPACE_DIR") {
        PathBuf::from(path)
    } else if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path).join("cix/workspaces")
    } else {
        env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".cache/cix/workspaces")
    };
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn gitignore_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            if path.file_name().is_some_and(|name| name == ".gitignore") {
                files.push(path.clone());
            }
            if path.is_dir() {
                directories.push(path);
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::IgnoreSet;
    use std::{env, fs};

    #[test]
    fn context_ignores_cix_output_without_hiding_source() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
        fs::create_dir_all(root.join("ignored")).unwrap();
        let workspace = root.join("workspaces");
        let prior = env::var_os("CIX_BUILD_WORKSPACE_DIR");
        env::set_var("CIX_BUILD_WORKSPACE_DIR", &workspace);
        let ignores = IgnoreSet::new(root);
        assert!(ignores.ignores(&root.join(".git/HEAD")));
        assert!(ignores.ignores(&root.join("target/debug/cix")));
        assert!(ignores.ignores(&root.join("Cixfile.lock")));
        assert!(ignores.ignores(&root.join("Cixfile.lock.tmp")));
        assert!(ignores.ignores(&root.join("cix.lock")));
        assert!(ignores.ignores(&workspace.join("x/work/state.json")));
        assert!(ignores.ignores(&root.join("ignored/source.txt")));
        assert!(ignores.ignores(root));
        assert!(!ignores.ignores(&root.join("start")));
        assert!(!ignores.ignores(&root.join("src/main.rs")));
        match prior {
            Some(value) => env::set_var("CIX_BUILD_WORKSPACE_DIR", value),
            None => env::remove_var("CIX_BUILD_WORKSPACE_DIR"),
        }
    }
}
