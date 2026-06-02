pub mod filesystem;
pub mod github;

#[derive(clap::Subcommand)]
pub enum Command {
    Filesystem(filesystem::Command),
    Github(github::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Filesystem(filesystem::Request),
    FilesystemRequestSchema(filesystem::request_schema::Request),
    FilesystemResponseSchema(filesystem::response_schema::Request),
    Github(github::Request),
    GithubRequestSchema(github::request_schema::Request),
    GithubResponseSchema(github::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Filesystem(filesystem::Response),
    FilesystemRequestSchema(filesystem::request_schema::Response),
    FilesystemResponseSchema(filesystem::response_schema::Response),
    Github(github::Response),
    GithubRequestSchema(github::request_schema::Response),
    GithubResponseSchema(github::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Filesystem(cmd) => match cmd.schema {
                None => Ok(Request::Filesystem(filesystem::Request::try_from(cmd.args)?)),
                Some(filesystem::Schema::RequestSchema(args)) =>
                    Ok(Request::FilesystemRequestSchema(filesystem::request_schema::Request::try_from(args)?)),
                Some(filesystem::Schema::ResponseSchema(args)) =>
                    Ok(Request::FilesystemResponseSchema(filesystem::response_schema::Request::try_from(args)?)),
            },
            Command::Github(cmd) => match cmd.schema {
                None => Ok(Request::Github(github::Request::try_from(cmd.args)?)),
                Some(github::Schema::RequestSchema(args)) =>
                    Ok(Request::GithubRequestSchema(github::request_schema::Request::try_from(args)?)),
                Some(github::Schema::ResponseSchema(args)) =>
                    Ok(Request::GithubResponseSchema(github::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Filesystem(inner) => inner.into_command(),
            Request::FilesystemRequestSchema(inner) => inner.into_command(),
            Request::FilesystemResponseSchema(inner) => inner.into_command(),
            Request::Github(inner) => inner.into_command(),
            Request::GithubRequestSchema(inner) => inner.into_command(),
            Request::GithubResponseSchema(inner) => inner.into_command(),
        }
    }
}
