pub mod audio;
pub mod file;
pub mod image;
pub mod text;
pub mod video;

#[derive(clap::Subcommand)]
pub enum Command {
    Audio {
        #[command(subcommand)]
        command: audio::Command,
    },
    File {
        #[command(subcommand)]
        command: file::Command,
    },
    Image {
        #[command(subcommand)]
        command: image::Command,
    },
    Text {
        #[command(subcommand)]
        command: text::Command,
    },
    Video {
        #[command(subcommand)]
        command: video::Command,
    },
}
