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
    /// Serve the tags under a root_url as an HTTP index.
    Serve {
        root_url: String,
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
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        anyhow::bail!("not implemented yet (index track)")
    }
}
