use std::path::PathBuf;

use crate::UpdateRequest;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Validate or compare a compose.json without activation.
    Compose {
        #[command(subcommand)]
        command: ComposeCommand,
    },
    /// Resolve, build, profile, and activate a composite.
    Up {
        /// Path to compose.json.
        #[arg(default_value = "compose.json")]
        file: PathBuf,
        /// Re-resolve all pinned services, or one named service.
        #[arg(long, num_args = 0..=1, default_missing_value = "*", value_name = "SERVICE")]
        update: Option<String>,
    },
    /// Stop and unlink a composite while retaining its profile.
    Down {
        /// Composite name; defaults to the name in ./compose.json.
        name: Option<String>,
    },
    /// Roll a composite profile back one generation and activate it.
    Rollback {
        /// Composite name.
        name: String,
    },
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
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Compose {
                command: ComposeCommand::Check { file },
            } => crate::check(&file),
            Self::Compose {
                command: ComposeCommand::Diff { file },
            } => crate::diff(&file),
            Self::Up { file, update } => {
                let update = match update.as_deref() {
                    None => UpdateRequest::None,
                    Some("*") => UpdateRequest::All,
                    Some(service) => UpdateRequest::Service(service.to_owned()),
                };
                crate::up(&file, update)
            }
            Self::Down { name } => {
                let name = match name {
                    Some(name) => name,
                    None => crate::Compose::load(&PathBuf::from("compose.json"))?.name,
                };
                crate::down(&name)
            }
            Self::Rollback { name } => crate::rollback(&name),
        }
    }
}
