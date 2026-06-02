pub mod add;
pub mod del;
pub mod edit;
pub mod get;

#[derive(clap::Subcommand)]
pub enum Command {
    Add(add::Command),
    Del(del::Command),
    Edit(edit::Command),
    Get(get::Command),
}
