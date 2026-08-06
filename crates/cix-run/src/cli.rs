#[derive(clap::Subcommand)]
pub enum Command {
    /// Run cix's native HTTP/TCP readiness and watchdog adapters.
    Probe {
        #[command(subcommand)]
        command: crate::probe::Command,
    },
    /// Run a manifested service as a transient systemd unit.
    Run {
        /// Store path or flake installable, optionally with `#service`.
        ///
        /// For a flake output, append a second suffix: `flake#package#service`.
        #[arg(required_unless_present = "compose", conflicts_with = "compose")]
        installable: Option<String>,
        /// Run an anonymous compose JSON document (a path or '-' for stdin).
        #[arg(long, value_name = "FILE|-")]
        compose: Option<std::path::PathBuf>,
        /// Override a declared environment variable (`NAME=VALUE`).
        #[arg(short = 'e', long = "env", value_name = "NAME=VALUE")]
        env: Vec<String>,
        /// Override a named port (`NAME=PORT`) or bind a listener (`NAME=ADDR:PORT`).
        #[arg(short = 'p', long = "port", value_name = "NAME=VALUE")]
        port: Vec<String>,
        /// Materialize a declared directory (`/path=host:/host/path`, `shared:name`, or `as:state`).
        #[arg(long = "dir", value_name = "PATH=MATERIALIZATION")]
        dirs: Vec<String>,
        /// Stable host identity required by host-backed directories.
        #[arg(long)]
        identity: Option<String>,
        /// Print the transient unit name and return without following logs.
        #[arg(long)]
        detach: bool,
        /// Schedule an app with systemd's raw OnCalendar syntax.
        #[arg(long, value_name = "ON_CALENDAR")]
        schedule: Option<String>,
        /// Audit the service in a sealed filesystem root (CIP-84 phase 1).
        #[arg(long)]
        closed_root: bool,
        /// Degraded dev mode against the user manager (no DynamicUser).
        #[arg(long)]
        user: bool,
    },
    /// Open a shell or run a command in a fresh copy of a service sandbox.
    Debug {
        /// Store path or flake installable, optionally with `#service`.
        installable: String,
        /// Override a declared environment variable (`NAME=VALUE`).
        #[arg(short = 'e', long = "env", value_name = "NAME=VALUE")]
        env: Vec<String>,
        /// Degraded dev mode against the user manager (no DynamicUser).
        #[arg(long)]
        user: bool,
        /// Command to run instead of the service-PATH shell.
        #[arg(last = true, value_name = "COMMAND")]
        command: Vec<String>,
    },
    /// Run a command inside a live cix service's private namespaces.
    Exec {
        /// Exact unit name from `cix ps`, or an unambiguous running service name.
        target: String,
        /// Keep root identity after joining the service's private namespaces.
        #[arg(long)]
        root: bool,
        /// Degraded mode: use a user unit's environment without joining namespaces.
        #[arg(long)]
        user: bool,
        /// Command to run instead of the unit-PATH shell.
        #[arg(last = true, value_name = "COMMAND")]
        command: Vec<String>,
    },
    /// List running cix-* units.
    Ps {
        /// Render stable machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    #[command(hide = true)]
    ClosedRootNss {
        identity: String,
        directory: std::path::PathBuf,
    },
}

impl Command {
    pub fn run(self, resolver: &dyn crate::InstallableResolver) -> anyhow::Result<()> {
        match self {
            Self::Probe { command } => command.run(),
            Self::Run {
                installable,
                compose,
                env,
                port,
                dirs,
                identity,
                detach,
                schedule,
                closed_root,
                user,
            } => {
                if compose.is_some() {
                    anyhow::bail!("cix run --compose is handled by the top-level cix command")
                }
                crate::runtime::run(
                    crate::runtime::RunOptions {
                        installable: installable
                            .expect("clap requires installable without --compose"),
                        env,
                        port,
                        dirs,
                        identity,
                        detach,
                        schedule,
                        closed_root,
                        user,
                    },
                    resolver,
                )
            }
            Self::Debug {
                installable,
                env,
                user,
                command,
            } => crate::debug::debug(
                crate::debug::DebugOptions {
                    installable,
                    env,
                    user,
                    command,
                },
                resolver,
            ),
            Self::Exec {
                target,
                root,
                user,
                command,
            } => crate::exec::exec(crate::exec::ExecOptions {
                target,
                root,
                user,
                command,
            }),
            Self::Ps { .. } => anyhow::bail!("cix ps is handled by the top-level cix command"),
            Self::ClosedRootNss {
                identity,
                directory,
            } => crate::closed_root::write_nss_for_directory(&identity, &directory),
        }
    }
}
