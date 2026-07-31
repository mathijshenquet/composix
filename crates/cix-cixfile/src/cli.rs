use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::BuildOptions;

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
        }
    }
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
