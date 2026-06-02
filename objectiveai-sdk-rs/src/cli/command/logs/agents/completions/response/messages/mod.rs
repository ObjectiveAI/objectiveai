pub mod audio;
pub mod clear;
pub mod file;
pub mod get;
pub mod image;
pub mod logprobs;
pub mod reasoning;
pub mod refusal;
pub mod subscribe;
pub mod text;
pub mod tool;
pub mod tool_calls;
pub mod video;

#[derive(clap::Subcommand)]
pub enum Command {
    Audio {
        #[command(subcommand)]
        command: audio::Command,
    },
    Clear(clear::Command),
    File {
        #[command(subcommand)]
        command: file::Command,
    },
    Get(get::Command),
    Image {
        #[command(subcommand)]
        command: image::Command,
    },
    Logprobs {
        #[command(subcommand)]
        command: logprobs::Command,
    },
    Reasoning {
        #[command(subcommand)]
        command: reasoning::Command,
    },
    Refusal {
        #[command(subcommand)]
        command: refusal::Command,
    },
    Subscribe(subscribe::Command),
    Text {
        #[command(subcommand)]
        command: text::Command,
    },
    Tool {
        #[command(subcommand)]
        command: tool::Command,
    },
    ToolCalls {
        #[command(subcommand)]
        command: tool_calls::Command,
    },
    Video {
        #[command(subcommand)]
        command: video::Command,
    },
}
