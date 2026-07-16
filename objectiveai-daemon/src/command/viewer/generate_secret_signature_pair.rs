//! `viewer generate-secret-signature-pair` — emit a fresh random
//! `(secret, signature)` pair for viewer authentication. Pure local
//! computation; no IO.

use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let pair = crate::filesystem::config::generate_viewer_secret_signature_pair();
    Ok(Response {
        secret: pair.secret,
        signature: pair.signature,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
