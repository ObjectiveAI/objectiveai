pub mod request;
pub mod response;

#[derive(clap::Subcommand)]
pub enum Command {
    Request {
        #[command(subcommand)]
        command: request::Command,
    },
    Response {
        #[command(subcommand)]
        command: response::Command,
    },
}
