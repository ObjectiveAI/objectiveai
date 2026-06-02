pub mod clear;
pub mod get;
pub mod list;
pub mod retry_tokens;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Get(get::Command),
    List(list::Command),
    RetryTokens {
        #[command(subcommand)]
        command: retry_tokens::Command,
    },
    Subscribe(subscribe::Command),
}
