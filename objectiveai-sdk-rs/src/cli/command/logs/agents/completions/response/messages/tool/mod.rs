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

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Audio(audio::Request),
    File(file::Request),
    Image(image::Request),
    Text(text::Request),
    Video(video::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Response {
    Audio(audio::Response),
    File(file::Response),
    Image(image::Response),
    Text(text::Response),
    Video(video::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Audio { command } =>
                Ok(Request::Audio(audio::Request::try_from(command)?)),
            Command::File { command } =>
                Ok(Request::File(file::Request::try_from(command)?)),
            Command::Image { command } =>
                Ok(Request::Image(image::Request::try_from(command)?)),
            Command::Text { command } =>
                Ok(Request::Text(text::Request::try_from(command)?)),
            Command::Video { command } =>
                Ok(Request::Video(video::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Audio(inner) => inner.into_command(),
            Request::File(inner) => inner.into_command(),
            Request::Image(inner) => inner.into_command(),
            Request::Text(inner) => inner.into_command(),
            Request::Video(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Audio(req) => {
                let inner = audio::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(Response::Audio)))
            }
            Request::File(req) => {
                let inner = file::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(Response::File)))
            }
            Request::Image(req) => {
                let inner = image::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(Response::Image)))
            }
            Request::Text(req) => {
                let inner = text::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(Response::Text)))
            }
            Request::Video(req) => {
                let inner = video::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(Response::Video)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    jq: String,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Audio(req) => {
                let inner = audio::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::File(req) => {
                let inner = file::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Image(req) => {
                let inner = image::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Text(req) => {
                let inner = text::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Video(req) => {
                let inner = video::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
