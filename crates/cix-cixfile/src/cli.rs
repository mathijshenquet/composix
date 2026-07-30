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
        /// Re-run build steps without persistent CACHE directories.
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
                no_cache,
            } => {
                let items = crate::build(&BuildOptions {
                    directory: dir,
                    update_lock,
                    tag,
                    no_cache,
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
