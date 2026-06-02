pub mod kill;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    Kill(kill::Command),
    Spawn(spawn::Command),
}
