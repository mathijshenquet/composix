use std::path::PathBuf;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Import a Docker archive or OCI layout (experimental compatibility prototype).
    Import {
        /// Path to a `docker save` tarball or an OCI image-layout directory.
        input: PathBuf,
        /// Override the generated service and store-item name.
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Import { input, name } => {
                let imported = crate::import(&input, name.as_deref())?;
                for finding in &imported.findings {
                    eprintln!("warning: {finding}");
                }
                println!("{}", imported.store_path.display());
                Ok(())
            }
        }
    }
}
