pub mod get;
pub mod install;
pub mod list;
pub mod run;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Install(install::Command),
    List(list::Command),
    Run(run::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Install(install::Request),
    InstallRequestSchema(install::request_schema::Request),
    InstallResponseSchema(install::response_schema::Request),
    List(list::Request),
    ListRequestSchema(list::request_schema::Request),
    ListResponseSchema(list::response_schema::Request),
    Run(run::Request),
    RunRequestSchema(run::request_schema::Request),
    RunResponseSchema(run::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Install(install::Response),
    InstallRequestSchema(install::request_schema::Response),
    InstallResponseSchema(install::response_schema::Response),
    List(list::ResponseItem),
    ListRequestSchema(list::request_schema::Response),
    ListResponseSchema(list::response_schema::Response),
    Run(run::ResponseItem),
    RunRequestSchema(run::request_schema::Response),
    RunResponseSchema(run::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Install(cmd) => match cmd.schema {
                None => Ok(Request::Install(install::Request::try_from(cmd.args)?)),
                Some(install::Schema::RequestSchema(args)) =>
                    Ok(Request::InstallRequestSchema(install::request_schema::Request::try_from(args)?)),
                Some(install::Schema::ResponseSchema(args)) =>
                    Ok(Request::InstallResponseSchema(install::response_schema::Request::try_from(args)?)),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::Run(cmd) => match cmd.schema {
                None => Ok(Request::Run(run::Request::try_from(cmd.args)?)),
                Some(run::Schema::RequestSchema(args)) =>
                    Ok(Request::RunRequestSchema(run::request_schema::Request::try_from(args)?)),
                Some(run::Schema::ResponseSchema(args)) =>
                    Ok(Request::RunResponseSchema(run::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Install(inner) => inner.into_command(),
            Request::InstallRequestSchema(inner) => inner.into_command(),
            Request::InstallResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Run(inner) => inner.into_command(),
            Request::RunRequestSchema(inner) => inner.into_command(),
            Request::RunResponseSchema(inner) => inner.into_command(),
        }
    }
}
