#[derive(clap::Subcommand)]
pub enum Command {
    /// Run a spec'ed service as a transient systemd unit.
    Run {
        /// Store path or flake installable, optionally with `#service`.
        ///
        /// For a flake output, append a second suffix: `flake#package#service`.
        installable: String,
        /// Override a declared environment variable (`NAME=VALUE`).
        #[arg(short = 'e', long = "env", value_name = "NAME=VALUE")]
        env: Vec<String>,
        /// Override a named port (`NAME=PORT`) or bind a listener (`NAME=ADDR:PORT`).
        #[arg(short = 'p', long = "port", value_name = "NAME=VALUE")]
        port: Vec<String>,
        /// Print the transient unit name and return without following logs.
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
        match self {
            Self::Run {
                installable,
                env,
                port,
                detach,
                user,
            } => crate::runtime::run(crate::runtime::RunOptions {
                installable,
                env,
                port,
                detach,
                user,
            }),
            Self::Ps => crate::runtime::ps(),
        }
    }
}
