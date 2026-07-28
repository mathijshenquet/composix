#[derive(clap::Subcommand)]
pub enum Command {
    /// Run a spec'ed service as a transient systemd unit.
    Run {
        /// Installable, optionally with `#service` when the spec has several.
        installable: String,
        #[arg(short, long)]
        env: Vec<String>,
        #[arg(short, long)]
        port: Vec<String>,
        #[arg(long)]
        detach: bool,
        /// Degraded dev mode against the user manager (no DynamicUser).
        #[arg(long)]
        user: bool,
    },
    /// List running cix-* units.
    Ps,
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        anyhow::bail!("not implemented yet (run track)")
    }
}
