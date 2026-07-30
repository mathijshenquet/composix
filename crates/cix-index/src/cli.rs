#[derive(clap::Subcommand)]
pub enum Command {
    /// Tag an installable (store path, flake installable, or existing ref).
    Tag { installable: String, r#ref: String },
    /// Remove a tag (unpins its GC root).
    Untag { r#ref: String },
    /// List tags.
    Ls {
        prefix: Option<String>,
        #[arg(short, long)]
        long: bool,
    },
    /// Serve bare local tags as an HTTP index.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8420")]
        listen: String,
        #[arg(long)]
        substituter: Vec<String>,
        #[arg(long)]
        with_store: bool,
        #[arg(long)]
        sign_key: Option<String>,
    },
    /// Pull a remote ref (or refresh all upstreams when no ref given).
    Pull {
        r#ref: Option<String>,
        #[arg(long)]
        r#as: Option<String>,
    },
    /// Show the available immutable table history for one name.
    History { name: String },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Tag { installable, r#ref } => crate::tag(&installable, &r#ref, None),
            Self::Untag { r#ref } => crate::untag(&r#ref),
            Self::Ls { prefix, long } => {
                let listing = crate::list(prefix.as_deref(), long)?;
                if !listing.is_empty() {
                    println!("{listing}");
                }
                Ok(())
            }
            Self::Serve {
                listen,
                substituter,
                with_store,
                sign_key,
            } => crate::serve(&listen, substituter, with_store, sign_key.as_deref()),
            Self::Pull { r#ref, r#as } => {
                let updated = crate::pull(r#ref.as_deref(), r#as.as_deref())?;
                println!("updated {updated} tag(s)");
                Ok(())
            }
            Self::History { name } => {
                for entry in crate::history(&name)? {
                    println!("{} {}", entry.nar_hash, entry.tags.join(","));
                }
                Ok(())
            }
        }
    }
}
