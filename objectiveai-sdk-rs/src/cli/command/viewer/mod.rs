pub mod generate_secret_signature_pair;
pub mod kill;
pub mod send;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    GenerateSecretSignaturePair(generate_secret_signature_pair::Command),
    Kill(kill::Command),
    Send(send::Command),
    Spawn(spawn::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    GenerateSecretSignaturePair(generate_secret_signature_pair::Request),
    GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Request),
    GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Request),
    Kill(kill::Request),
    KillRequestSchema(kill::request_schema::Request),
    KillResponseSchema(kill::response_schema::Request),
    Send(send::Request),
    SendRequestSchema(send::request_schema::Request),
    SendResponseSchema(send::response_schema::Request),
    Spawn(spawn::Request),
    SpawnRequestSchema(spawn::request_schema::Request),
    SpawnResponseSchema(spawn::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    GenerateSecretSignaturePair(generate_secret_signature_pair::Response),
    GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Response),
    GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Response),
    Kill(kill::Response),
    KillRequestSchema(kill::request_schema::Response),
    KillResponseSchema(kill::response_schema::Response),
    Send(send::Response),
    SendRequestSchema(send::request_schema::Response),
    SendResponseSchema(send::response_schema::Response),
    Spawn(spawn::Response),
    SpawnRequestSchema(spawn::request_schema::Response),
    SpawnResponseSchema(spawn::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::GenerateSecretSignaturePair(cmd) => match cmd.schema {
                None => Ok(Request::GenerateSecretSignaturePair(generate_secret_signature_pair::Request::try_from(cmd.args)?)),
                Some(generate_secret_signature_pair::Schema::RequestSchema(args)) =>
                    Ok(Request::GenerateSecretSignaturePairRequestSchema(generate_secret_signature_pair::request_schema::Request::try_from(args)?)),
                Some(generate_secret_signature_pair::Schema::ResponseSchema(args)) =>
                    Ok(Request::GenerateSecretSignaturePairResponseSchema(generate_secret_signature_pair::response_schema::Request::try_from(args)?)),
            },
            Command::Kill(cmd) => match cmd.schema {
                None => Ok(Request::Kill(kill::Request::try_from(cmd.args)?)),
                Some(kill::Schema::RequestSchema(args)) =>
                    Ok(Request::KillRequestSchema(kill::request_schema::Request::try_from(args)?)),
                Some(kill::Schema::ResponseSchema(args)) =>
                    Ok(Request::KillResponseSchema(kill::response_schema::Request::try_from(args)?)),
            },
            Command::Send(cmd) => match cmd.schema {
                None => Ok(Request::Send(send::Request::try_from(cmd.args)?)),
                Some(send::Schema::RequestSchema(args)) =>
                    Ok(Request::SendRequestSchema(send::request_schema::Request::try_from(args)?)),
                Some(send::Schema::ResponseSchema(args)) =>
                    Ok(Request::SendResponseSchema(send::response_schema::Request::try_from(args)?)),
            },
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::GenerateSecretSignaturePair(inner) => inner.into_command(),
            Request::GenerateSecretSignaturePairRequestSchema(inner) => inner.into_command(),
            Request::GenerateSecretSignaturePairResponseSchema(inner) => inner.into_command(),
            Request::Kill(inner) => inner.into_command(),
            Request::KillRequestSchema(inner) => inner.into_command(),
            Request::KillResponseSchema(inner) => inner.into_command(),
            Request::Send(inner) => inner.into_command(),
            Request::SendRequestSchema(inner) => inner.into_command(),
            Request::SendResponseSchema(inner) => inner.into_command(),
            Request::Spawn(inner) => inner.into_command(),
            Request::SpawnRequestSchema(inner) => inner.into_command(),
            Request::SpawnResponseSchema(inner) => inner.into_command(),
        }
    }
}
