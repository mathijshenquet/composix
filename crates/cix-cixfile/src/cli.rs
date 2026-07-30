use std::path::PathBuf;

use crate::BuildOptions;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Build a Cixfile directory into a runnable Nix store item.
    Build {
        #[arg(default_value = ".")]
        dir: PathBuf,
        #[arg(short = 't', long)]
        tag: Option<String>,
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
                update_lock,
                cold,
                no_cache,
            } => {
                if no_cache {
                    eprintln!("warning: --no-cache is deprecated; use --cold");
                }
                let items = crate::build(&BuildOptions {
                    directory: dir,
                    update_lock,
                    tag,
                    cold: cold || no_cache,
                })?;
                if items.len() == 1 {
                    println!("{}", items[0].store_path);
                } else {
                    for item in items {
                        println!("{} {}", item.name, item.store_path);
                    }
                }
                Ok(())
            }
        }
    }
}
