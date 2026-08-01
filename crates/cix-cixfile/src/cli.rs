use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::BuildOptions;
use anyhow::{bail, Context};
use ignore::WalkBuilder;
use similar::TextDiff;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Build a Cixfile directory into a runnable Nix store item.
    Build {
        #[arg(default_value = ".")]
        dir: String,
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
        /// Deprecated alias for --cold.
        #[arg(long)]
        no_cache: bool,
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
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Build {
                dir,
                tag,
                namespace,
                update_lock,
                cold,
                no_cache,
            } => {
                if no_cache {
                    eprintln!("warning: --no-cache is deprecated; use --cold");
                }
                let (directory, selector) = parse_build_target(&dir)?;
                let options = BuildOptions {
                    directory,
                    update_lock,
                    tag: None,
                    cold: cold || no_cache,
                };
                let items =
                    crate::build_family(&options, &tag, namespace.as_deref(), selector.as_deref())?;
                if selector.is_some() {
                    println!("{}", items[0].store_path);
                } else {
                    let members = items
                        .into_iter()
                        .map(|item| (item.name, item.store_path))
                        .collect::<BTreeMap<_, _>>();
                    println!("{}", serde_json::to_string(&members)?);
                }
                Ok(())
            }
            Self::Fmt { paths, check } => format_paths(paths, check),
            Self::Watch { path } => crate::watch(&path),
        }
    }
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
        let formatted = crate::fmt::format(&input)?;
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
        let formatted = crate::fmt::format(&input)?;
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
    use super::parse_build_target;

    #[test]
    fn parses_member_selector_from_build_target() {
        assert_eq!(
            parse_build_target(".#api").unwrap(),
            (".".into(), Some("api".into()))
        );
        assert_eq!(parse_build_target(".").unwrap(), (".".into(), None));
    }
}
