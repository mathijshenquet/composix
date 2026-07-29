#[derive(clap::Subcommand)]
pub enum Command {}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {}
    }
}
