use std::path::PathBuf;

use crate::UpdateRequest;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Validate or compare a compose.json without activation.
    Compose {
        #[command(subcommand)]
        command: ComposeCommand,
    },
    /// Edit the mutable host-root tree without rewriting JSON by hand.
    Root {
        #[command(subcommand)]
        command: RootCommand,
    },
    /// Resolve, build, profile, and activate a composite.
    Up {
        /// Path to compose.json.
        #[arg(default_value = "compose.json")]
        file: PathBuf,
        /// Re-resolve every ref, or one path and its subtree.
        #[arg(long = "update-lock", alias = "update", num_args = 0..=1, default_missing_value = "*", value_name = "PATH")]
        update_lock: Option<String>,
        /// Audit generated services in sealed filesystem roots (CIP-84 phase 1).
        #[arg(long)]
        closed_root: bool,
    },
    /// Stop and unlink a composite while retaining its profile.
    Down {
        /// Composite name; defaults to the name in ./compose.json.
        name: Option<String>,
        /// Delete cix-owned private and shared role data after confirmation.
        #[arg(long)]
        purge: bool,
        /// Confirm --purge without an interactive prompt.
        #[arg(long, requires = "purge")]
        yes: bool,
    },
    /// Remove explicitly expendable role data from a composite.
    Clean {
        /// Composite name.
        name: String,
        /// Directory class to clean.
        #[arg(long)]
        what: CleanWhat,
    },
    /// Refused: cix has no writable container layer to recreate.
    Recreate {
        /// Composite name retained only to make the migration command obvious.
        name: Option<String>,
    },
    /// Roll a composite profile back one generation and activate it.
    Rollback {
        /// Composite name.
        name: String,
    },
}

#[derive(clap::Subcommand)]
pub enum RootCommand {
    /// Add an item ref at a slash-separated child path.
    Add {
        path: String,
        reference: String,
        /// Host-root file to edit.
        #[arg(long, default_value = "/etc/cix/cix.json")]
        file: PathBuf,
    },
    /// Remove a child ref or inline subtree at a slash-separated path.
    Remove {
        path: String,
        /// Host-root file to edit.
        #[arg(long, default_value = "/etc/cix/cix.json")]
        file: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum CleanWhat {
    Cache,
    Logs,
    State,
    Dir,
    Shared,
}

#[derive(clap::Subcommand)]
pub enum ComposeCommand {
    /// Resolve and semantically validate compose.json.
    Check {
        /// Path to compose.json.
        #[arg(default_value = "compose.json")]
        file: PathBuf,
    },
    /// Dry-build and compare against the active profile generation.
    Diff {
        /// Path to compose.json.
        #[arg(default_value = "compose.json")]
        file: PathBuf,
        /// Compare a sealed-root generation (CIP-84 phase 1).
        #[arg(long)]
        closed_root: bool,
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Compose {
                command: ComposeCommand::Check { file },
            } => crate::check(&file),
            Self::Compose {
                command: ComposeCommand::Diff { file, closed_root },
            } => crate::diff(&file, closed_root),
            Self::Root {
                command: RootCommand::Add {
                    path,
                    reference,
                    file,
                },
            } => {
                let mut root = crate::Compose::load(&file)?;
                root.root_add(&path, reference)?;
                root.write(&file)?;
                println!("added {path} to {}", file.display());
                Ok(())
            }
            Self::Root {
                command: RootCommand::Remove { path, file },
            } => {
                let mut root = crate::Compose::load(&file)?;
                root.root_remove(&path)?;
                root.write(&file)?;
                println!("removed {path} from {}", file.display());
                Ok(())
            }
            Self::Up {
                file,
                update_lock,
                closed_root,
            } => {
                let update = match update_lock.as_deref() {
                    None => UpdateRequest::None,
                    Some("*") => UpdateRequest::All,
                    Some(path) => UpdateRequest::Path(path.to_owned()),
                };
                crate::up(&file, update, closed_root)
            }
            Self::Down { name, purge, yes } => {
                let name = match name {
                    Some(name) => name,
                    None => crate::Compose::load(&PathBuf::from("compose.json"))?.name,
                };
                crate::down(&name, purge, yes)
            }
            Self::Clean { name, what } => crate::clean(&name, what),
            Self::Recreate { name } => anyhow::bail!(
                "cix recreate is refused: cix up converges without a writable container layer; use cix clean --what=cache for expendable state{}",
                name.map(|name| format!(" in composite {name}")).unwrap_or_default()
            ),
            Self::Rollback { name } => crate::rollback(&name),
        }
    }
}
