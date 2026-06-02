pub mod alpha_scalar;
pub mod alpha_vector;
pub mod remote;

#[derive(clap::Subcommand)]
pub enum Command {
    AlphaScalar(alpha_scalar::Command),
    AlphaVector(alpha_vector::Command),
    Remote(remote::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    AlphaScalar(alpha_scalar::Request),
    AlphaScalarRequestSchema(alpha_scalar::request_schema::Request),
    AlphaScalarResponseSchema(alpha_scalar::response_schema::Request),
    AlphaVector(alpha_vector::Request),
    AlphaVectorRequestSchema(alpha_vector::request_schema::Request),
    AlphaVectorResponseSchema(alpha_vector::response_schema::Request),
    Remote(remote::Request),
    RemoteRequestSchema(remote::request_schema::Request),
    RemoteResponseSchema(remote::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    AlphaScalar(alpha_scalar::ResponseItem),
    AlphaScalarRequestSchema(alpha_scalar::request_schema::Response),
    AlphaScalarResponseSchema(alpha_scalar::response_schema::Response),
    AlphaVector(alpha_vector::ResponseItem),
    AlphaVectorRequestSchema(alpha_vector::request_schema::Response),
    AlphaVectorResponseSchema(alpha_vector::response_schema::Response),
    Remote(remote::ResponseItem),
    RemoteRequestSchema(remote::request_schema::Response),
    RemoteResponseSchema(remote::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::AlphaScalar(cmd) => match cmd.schema {
                None => Ok(Request::AlphaScalar(alpha_scalar::Request::try_from(cmd.args)?)),
                Some(alpha_scalar::Schema::RequestSchema(args)) =>
                    Ok(Request::AlphaScalarRequestSchema(alpha_scalar::request_schema::Request::try_from(args)?)),
                Some(alpha_scalar::Schema::ResponseSchema(args)) =>
                    Ok(Request::AlphaScalarResponseSchema(alpha_scalar::response_schema::Request::try_from(args)?)),
            },
            Command::AlphaVector(cmd) => match cmd.schema {
                None => Ok(Request::AlphaVector(alpha_vector::Request::try_from(cmd.args)?)),
                Some(alpha_vector::Schema::RequestSchema(args)) =>
                    Ok(Request::AlphaVectorRequestSchema(alpha_vector::request_schema::Request::try_from(args)?)),
                Some(alpha_vector::Schema::ResponseSchema(args)) =>
                    Ok(Request::AlphaVectorResponseSchema(alpha_vector::response_schema::Request::try_from(args)?)),
            },
            Command::Remote(cmd) => match cmd.schema {
                None => Ok(Request::Remote(remote::Request::try_from(cmd.args)?)),
                Some(remote::Schema::RequestSchema(args)) =>
                    Ok(Request::RemoteRequestSchema(remote::request_schema::Request::try_from(args)?)),
                Some(remote::Schema::ResponseSchema(args)) =>
                    Ok(Request::RemoteResponseSchema(remote::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::AlphaScalar(inner) => inner.into_command(),
            Request::AlphaScalarRequestSchema(inner) => inner.into_command(),
            Request::AlphaScalarResponseSchema(inner) => inner.into_command(),
            Request::AlphaVector(inner) => inner.into_command(),
            Request::AlphaVectorRequestSchema(inner) => inner.into_command(),
            Request::AlphaVectorResponseSchema(inner) => inner.into_command(),
            Request::Remote(inner) => inner.into_command(),
            Request::RemoteRequestSchema(inner) => inner.into_command(),
            Request::RemoteResponseSchema(inner) => inner.into_command(),
        }
    }
}
