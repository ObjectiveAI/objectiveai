//! `db spawn` — start the `objectiveai-db` postgres vehicle in the
//! background, using `config db` connection settings (built-in
//! defaults when unset).

use objectiveai_sdk::cli::command::db::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::{DB_DEFAULT_ADDRESS, DB_DEFAULT_PORT};

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;

    let db = config.db();
    let address = db
        .get_address()
        .unwrap_or(DB_DEFAULT_ADDRESS)
        .to_string();
    let port = db.get_port().unwrap_or(DB_DEFAULT_PORT);
    let password = db.get_password().map(String::from);

    // `objectiveai-db` is a launcher, not a resident server — it
    // starts the postmaster and exits — so an "already running" check
    // by process name would always pass. Probe the configured
    // address:port instead: anything listening there means the
    // database (or something else occupying its spot) is alive.
    if crate::spawn::tcp_alive(&address, port).await {
        return Err(Error::AlreadyListening { address, port });
    }

    // Forward state-root + password so the vehicle provisions inside
    // THIS cli's config base dir with the configured superuser
    // password. ADDRESS/PORT travel via the spawn helper itself.
    let mut extra_env: Vec<(&str, String)> = vec![(
        "CONFIG_BASE_DIR",
        ctx.filesystem.base_dir().to_string_lossy().into_owned(),
    )];
    if let Some(password) = password {
        extra_env.push(("PASSWORD", password));
    }

    let bin = if cfg!(windows) {
        "objectiveai-db.exe"
    } else {
        "objectiveai-db"
    };
    let exe = ctx.filesystem.base_dir().join("bin").join(bin);

    let listening =
        crate::spawn::spawn_and_wait_for_listening(&exe, &address, port, &extra_env).await?;
    Ok(Response { listening })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::spawn as sdk;
    use objectiveai_sdk::cli::command::db::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::spawn as sdk;
    use objectiveai_sdk::cli::command::db::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
