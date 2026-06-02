//! `viewer generate-secret-signature-pair` — emit a fresh random
//! `(secret, signature)` pair for viewer authentication. Pure local
//! computation; no IO.

use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    let pair = crate::filesystem::config::generate_viewer_secret_signature_pair();
    Ok(Response {
        secret: pair.secret,
        signature: pair.signature,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
