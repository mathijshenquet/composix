use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use cix_cixfile::BuildOptions;
use ignore::WalkBuilder;
use similar::TextDiff;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Build a Cixfile directory into a runnable Nix store item.
    Build {
        #[arg(default_value = ".")]
        dir: String,
        /// Named Cixfile in the build directory.
        #[arg(long, value_name = "NAME", default_value = "Cixfile")]
        file: String,
        #[arg(short = 't', long)]
        tag: Vec<String>,
        /// Family name used only when applying -t tags.
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long, num_args = 0..=1, default_missing_value = "")]
        update_lock: Option<String>,
        /// Re-run builders with empty persistent workspaces and verify consumed outputs.
        #[arg(long)]
        cold: bool,
        /// Keep build scratch directories and print their paths for debugging.
        #[arg(long)]
        keep_scratch: bool,
        /// Permit host-configured FETCH credentials without an interactive consent prompt (CI only).
        #[arg(long)]
        allow_secret: bool,
        /// Emit stable machine-readable per-step execution statistics.
        #[arg(long)]
        stats: bool,
        /// Directory containing persistent BUILDER workspaces.
        #[arg(long, env = "CIX_BUILD_WORKSPACE_DIR", default_value_os_t = cix_cixfile::default_workspace_directory())]
        workspace_directory: PathBuf,
    },
    /// Format Cixfiles.
    Fmt {
        /// Files or directories to format.
        paths: Vec<PathBuf>,
        /// Check formatting and print a diff without writing files.
        #[arg(long)]
        check: bool,
    },
    /// Rebuild Cixfile artifacts when their context changes.
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[command(flatten)]
        options: WatchArgs,
    },
}

impl Command {
    pub fn run(self, state_directory: &std::path::Path) -> anyhow::Result<()> {
        match self {
            Self::Build {
                dir,
                file,
                tag,
                namespace,
                update_lock,
                cold,
                keep_scratch,
                allow_secret,
                stats,
                workspace_directory,
            } => {
                cix_build::configure_scratch(keep_scratch);
                cix_build::install_signal_cleanup();
                let (directory, selector) = parse_build_target(&dir)?;
                let options = BuildOptions {
                    directory,
                    update_lock,
                    tag: None,
                    cold,
                    allow_secret,
                    workspace_directory,
                };
                let registry = crate::registry::IndexRegistry::open(state_directory.to_owned())?;
                let (items, build_stats) = cix_cixfile::build_family_with_stats_file_and_registry(
                    &options,
                    &tag,
                    namespace.as_deref(),
                    selector.as_deref(),
                    &file,
                    &registry,
                )?;
                if selector.is_some() {
                    if stats {
                        println!(
                            "{}",
                            render_json(
                                &serde_json::json!({ "item": items[0].store_path, "stats": build_stats })
                            )?
                        );
                    } else {
                        println!("{}", items[0].store_path);
                    }
                } else {
                    let members = items
                        .into_iter()
                        .map(|item| (item.name, item.store_path))
                        .collect::<BTreeMap<_, _>>();
                    if stats {
                        println!(
                            "{}",
                            render_json(
                                &serde_json::json!({ "items": members, "stats": build_stats })
                            )?
                        );
                    } else {
                        println!("{}", render_json(&members)?);
                    }
                }
                Ok(())
            }
            Self::Fmt { paths, check } => format_paths(paths, check),
            Self::Watch { path, options } => crate::watch::watch(
                &path,
                crate::watch::WatchOptions {
                    workspace_directory: options.workspace_directory,
                    debounce: std::time::Duration::from_millis(options.debounce_ms),
                    state_directory: state_directory.to_owned(),
                },
            ),
        }
    }
}

fn render_json(value: &impl serde::Serialize) -> anyhow::Result<String> {
    serde_json::to_string_pretty(value).context("serializing cix build JSON")
}

#[derive(clap::Args)]
pub struct WatchArgs {
    /// Directory containing persistent BUILDER workspaces.
    #[arg(long, env = "CIX_BUILD_WORKSPACE_DIR", default_value_os_t = cix_cixfile::default_workspace_directory())]
    workspace_directory: PathBuf,
    /// Delay in milliseconds before rebuilding a burst of changes.
    #[arg(long, env = "CIX_WATCH_DEBOUNCE_MS", default_value_t = 300)]
    debounce_ms: u64,
}

fn format_paths(paths: Vec<PathBuf>, check: bool) -> anyhow::Result<()> {
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    };
    let stdin = paths.iter().any(|path| path == Path::new("-"));
    if stdin && paths.len() != 1 {
        bail!("`-` (stdin) cannot be combined with file paths");
    }
    if stdin {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let formatted = cix_cixfile::fmt::format(&input)?;
        if check {
            if formatted != input {
                print_diff("-", &input, &formatted)?;
                bail!("formatting changes required");
            }
        } else {
            io::stdout().write_all(formatted.as_bytes())?;
        }
        return Ok(());
    }

    let files = discover_files(&paths)?;
    let mut changed = false;
    for path in files {
        let input =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let formatted = cix_cixfile::fmt::format(&input)?;
        if formatted == input {
            continue;
        }
        changed = true;
        if check {
            print_diff(&path.display().to_string(), &input, &formatted)?;
        } else {
            fs::write(&path, formatted).with_context(|| format!("writing {}", path.display()))?;
        }
    }
    if check && changed {
        bail!("formatting changes required");
    }
    Ok(())
}

fn discover_files(paths: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        let metadata = fs::metadata(path).with_context(|| format!("reading {}", path.display()))?;
        if metadata.is_file() {
            files.push(path.clone());
            continue;
        }
        let mut walker = WalkBuilder::new(path);
        walker
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false);
        for entry in walker.build() {
            let entry = entry?;
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && entry.file_name() == "Cixfile"
            {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn print_diff(name: &str, input: &str, formatted: &str) -> anyhow::Result<()> {
    let diff = TextDiff::from_lines(input, formatted)
        .unified_diff()
        .header(name, name)
        .to_string();
    io::stdout().write_all(diff.as_bytes())?;
    Ok(())
}

fn parse_build_target(input: &str) -> anyhow::Result<(PathBuf, Option<String>)> {
    match input.rsplit_once('#') {
        Some((directory, member)) => {
            if directory.is_empty() || member.is_empty() {
                anyhow::bail!("a Cixfile member selector is written <directory>#<member>")
            }
            Ok((PathBuf::from(directory), Some(member.to_owned())))
        }
        None => Ok((PathBuf::from(input), None)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{parse_build_target, render_json};

    #[test]
    fn parses_member_selector_from_build_target() {
        assert_eq!(
            parse_build_target(".#api").unwrap(),
            (".".into(), Some("api".into()))
        );
        assert_eq!(parse_build_target(".").unwrap(), (".".into(), None));
    }

    #[test]
    fn renders_build_json_with_indentation() {
        let output = render_json(&BTreeMap::from([("api", "/nix/store/api")])).unwrap();

        assert_eq!(output, "{\n  \"api\": \"/nix/store/api\"\n}");
    }
}
