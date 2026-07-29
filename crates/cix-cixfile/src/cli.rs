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
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Build {
                dir,
                tag,
                update_lock,
            } => {
                let store_path = crate::build(&BuildOptions {
                    directory: dir,
                    update_lock,
                    tag,
                })?;
                println!("{store_path}");
                Ok(())
            }
        }
    }
}
