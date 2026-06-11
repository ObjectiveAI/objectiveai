//! `viewer spawn` — start the `objectiveai-viewer` Tauri shell in the
//! background, using `viewer.address` + `viewer.port` from on-disk
//! config.

use objectiveai_sdk::cli::command::viewer::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;

    let address = config
        .viewer()
        .get_address()
        .ok_or(Error::MissingArgs(
            "viewer.address unset; run `objectiveai config viewer address set <addr>`",
        ))?
        .to_string();
    let port = config.viewer().get_port().ok_or(Error::MissingArgs(
        "viewer.port unset; run `objectiveai config viewer port set <port>`",
    ))?;

    crate::spawn::ensure_not_running("objectiveai-viewer")?;

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);

    let listening = crate::spawn::spawn_and_wait_for_listening(
        &exe,
        &address,
        port,
        &[
            (
                "OBJECTIVEAI_DIR",
                ctx.filesystem.dir().to_string_lossy().into_owned(),
            ),
            ("OBJECTIVEAI_STATE", ctx.filesystem.state().to_string()),
        ],
    )
    .await?;
    Ok(Response { listening })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
