pub mod get;
pub mod install;
pub mod list;
pub mod run;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Install {
        #[command(subcommand)]
        command: install::Command,
    },
    List(list::Command),
    Run(run::Command),
}
