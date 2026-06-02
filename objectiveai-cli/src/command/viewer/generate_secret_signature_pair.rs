//! `viewer generate-secret-signature-pair` — bare-naked handler stub.

use objectiveai_sdk::cli::command::viewer::generate_secret_signature_pair::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("viewer generate-secret-signature-pair execute")
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
