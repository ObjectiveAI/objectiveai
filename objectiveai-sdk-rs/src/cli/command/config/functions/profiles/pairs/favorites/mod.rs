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

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Add(add::Request),
    AddRequestSchema(add::request_schema::Request),
    AddResponseSchema(add::response_schema::Request),
    Del(del::Request),
    DelRequestSchema(del::request_schema::Request),
    DelResponseSchema(del::response_schema::Request),
    Edit(edit::Request),
    EditRequestSchema(edit::request_schema::Request),
    EditResponseSchema(edit::response_schema::Request),
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Add(add::Response),
    AddRequestSchema(add::request_schema::Response),
    AddResponseSchema(add::response_schema::Response),
    Del(del::Response),
    DelRequestSchema(del::request_schema::Response),
    DelResponseSchema(del::response_schema::Response),
    Edit(edit::Response),
    EditRequestSchema(edit::request_schema::Response),
    EditResponseSchema(edit::response_schema::Response),
    Get(get::ResponseItem),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Add(cmd) => match cmd.schema {
                None => Ok(Request::Add(add::Request::try_from(cmd.args)?)),
                Some(add::Schema::RequestSchema(args)) =>
                    Ok(Request::AddRequestSchema(add::request_schema::Request::try_from(args)?)),
                Some(add::Schema::ResponseSchema(args)) =>
                    Ok(Request::AddResponseSchema(add::response_schema::Request::try_from(args)?)),
            },
            Command::Del(cmd) => match cmd.schema {
                None => Ok(Request::Del(del::Request::try_from(cmd.args)?)),
                Some(del::Schema::RequestSchema(args)) =>
                    Ok(Request::DelRequestSchema(del::request_schema::Request::try_from(args)?)),
                Some(del::Schema::ResponseSchema(args)) =>
                    Ok(Request::DelResponseSchema(del::response_schema::Request::try_from(args)?)),
            },
            Command::Edit(cmd) => match cmd.schema {
                None => Ok(Request::Edit(edit::Request::try_from(cmd.args)?)),
                Some(edit::Schema::RequestSchema(args)) =>
                    Ok(Request::EditRequestSchema(edit::request_schema::Request::try_from(args)?)),
                Some(edit::Schema::ResponseSchema(args)) =>
                    Ok(Request::EditResponseSchema(edit::response_schema::Request::try_from(args)?)),
            },
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Add(inner) => inner.into_command(),
            Request::AddRequestSchema(inner) => inner.into_command(),
            Request::AddResponseSchema(inner) => inner.into_command(),
            Request::Del(inner) => inner.into_command(),
            Request::DelRequestSchema(inner) => inner.into_command(),
            Request::DelResponseSchema(inner) => inner.into_command(),
            Request::Edit(inner) => inner.into_command(),
            Request::EditRequestSchema(inner) => inner.into_command(),
            Request::EditResponseSchema(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
        }
    }
}
