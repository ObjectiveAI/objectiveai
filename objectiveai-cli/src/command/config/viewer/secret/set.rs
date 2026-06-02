//! `config viewer secret set` — bare-naked handler stub.

use objectiveai_sdk::cli::command::config::viewer::secret::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("config viewer secret set execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::viewer::secret::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::secret::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::viewer::secret::set as sdk;
    use objectiveai_sdk::cli::command::config::viewer::secret::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
