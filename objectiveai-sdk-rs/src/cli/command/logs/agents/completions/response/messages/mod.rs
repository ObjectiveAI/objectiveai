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

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Audio(audio::Request),
    Clear(clear::Request),
    ClearRequestSchema(clear::request_schema::Request),
    ClearResponseSchema(clear::response_schema::Request),
    File(file::Request),
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Image(image::Request),
    Logprobs(logprobs::Request),
    Reasoning(reasoning::Request),
    Refusal(refusal::Request),
    Subscribe(subscribe::Request),
    SubscribeRequestSchema(subscribe::request_schema::Request),
    SubscribeResponseSchema(subscribe::response_schema::Request),
    Text(text::Request),
    Tool(tool::Request),
    ToolCalls(tool_calls::Request),
    Video(video::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Audio(audio::Response),
    Clear(clear::Response),
    ClearRequestSchema(clear::request_schema::Response),
    ClearResponseSchema(clear::response_schema::Response),
    File(file::Response),
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Image(image::Response),
    Logprobs(logprobs::Response),
    Reasoning(reasoning::Response),
    Refusal(refusal::Response),
    Subscribe(subscribe::Response),
    SubscribeRequestSchema(subscribe::request_schema::Response),
    SubscribeResponseSchema(subscribe::response_schema::Response),
    Text(text::Response),
    Tool(tool::Response),
    ToolCalls(tool_calls::Response),
    Video(video::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Audio { command } =>
                Ok(Request::Audio(audio::Request::try_from(command)?)),
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::File { command } =>
                Ok(Request::File(file::Request::try_from(command)?)),
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Image { command } =>
                Ok(Request::Image(image::Request::try_from(command)?)),
            Command::Logprobs { command } =>
                Ok(Request::Logprobs(logprobs::Request::try_from(command)?)),
            Command::Reasoning { command } =>
                Ok(Request::Reasoning(reasoning::Request::try_from(command)?)),
            Command::Refusal { command } =>
                Ok(Request::Refusal(refusal::Request::try_from(command)?)),
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) =>
                    Ok(Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?)),
                Some(subscribe::Schema::ResponseSchema(args)) =>
                    Ok(Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?)),
            },
            Command::Text { command } =>
                Ok(Request::Text(text::Request::try_from(command)?)),
            Command::Tool { command } =>
                Ok(Request::Tool(tool::Request::try_from(command)?)),
            Command::ToolCalls { command } =>
                Ok(Request::ToolCalls(tool_calls::Request::try_from(command)?)),
            Command::Video { command } =>
                Ok(Request::Video(video::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Audio(inner) => inner.into_command(),
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::File(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Image(inner) => inner.into_command(),
            Request::Logprobs(inner) => inner.into_command(),
            Request::Reasoning(inner) => inner.into_command(),
            Request::Refusal(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
            Request::Text(inner) => inner.into_command(),
            Request::Tool(inner) => inner.into_command(),
            Request::ToolCalls(inner) => inner.into_command(),
            Request::Video(inner) => inner.into_command(),
        }
    }
}
