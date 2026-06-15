//! `tools install filesystem` — return the static INSTRUCTIONS.md
//! asset describing how to install a tool from a local path.

use objectiveai_sdk::cli::command::tools::install::filesystem::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        instructions: include_str!(
            "../../../../assets/tools/install/filesystem/INSTRUCTIONS.md"
        )
        .to_string(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::install::filesystem as sdk;
    use objectiveai_sdk::cli::command::tools::install::filesystem::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::install::filesystem as sdk;
    use objectiveai_sdk::cli::command::tools::install::filesystem::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
