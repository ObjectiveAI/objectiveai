//! `agents queue read id` — resolve a `prompt_contents.id` to its
//! typed payload via the per-kind content tables.

use objectiveai_sdk::cli::command::agents::queue::read::id::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db::{self, prompts::ContentRow};

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let row = db::prompts::read_content_async(ctx.filesystem.clone(), request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "queue content id {}",
                request.id
            )))
        })?;
    Ok(match row {
        ContentRow::Text(text) => Response::Text { text },
        ContentRow::Image(image_url) => Response::Image { image_url },
        ContentRow::Audio(input_audio) => Response::Audio { input_audio },
        ContentRow::Video(video_url) => Response::Video { video_url },
        ContentRow::File(file) => Response::File { file },
        ContentRow::Reasoning(reasoning) => Response::Reasoning { reasoning },
        ContentRow::Refusal(refusal) => Response::Refusal { refusal },
        ContentRow::ToolCall(tool_call) => Response::ToolCall { tool_call },
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::queue::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::id::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::queue::read::id as sdk;
    use objectiveai_sdk::cli::command::agents::queue::read::id::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
