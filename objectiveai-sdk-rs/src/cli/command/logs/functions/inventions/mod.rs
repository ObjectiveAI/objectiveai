pub mod recursive;
pub mod request;
pub mod response;

#[derive(clap::Subcommand)]
pub enum Command {
    Recursive {
        #[command(subcommand)]
        command: recursive::Command,
    },
    Request {
        #[command(subcommand)]
        command: request::Command,
    },
    Response {
        #[command(subcommand)]
        command: response::Command,
    },
}
