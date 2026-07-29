use clap::Parser;

/// composix: a docker-shaped toolkit on nix + systemd.
#[derive(Parser)]
#[command(name = "cix", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[command(flatten)]
    Cixfile(cix_cixfile::cli::Command),
    #[command(flatten)]
    Compose(cix_compose::cli::Command),
    #[command(flatten)]
    Index(cix_index::cli::Command),
    #[command(flatten)]
    Run(cix_run::cli::Command),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Cixfile(cmd) => cmd.run(),
        Command::Compose(cmd) => cmd.run(),
        Command::Index(cmd) => cmd.run(),
        Command::Run(cix_run::cli::Command::Ps) => cix_compose::ps(),
        Command::Run(cmd) => cmd.run(),
    }
}
