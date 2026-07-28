//! `development plugins mcp create` — register a local directory for a
//! plugin's coordinates.

use objectiveai_sdk::cli::command::development::plugins::mcp::create::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins mcp create requires the resident daemon".to_string(),
        )
    })?;

    // Absolute, because the LABORATORY HOST resolves this path, and its
    // working directory is neither this process's nor the caller's.
    // Rejected here rather than at build time so the mistake surfaces
    // when it is made, not on the next completion.
    let path = std::path::PathBuf::from(&request.path);
    if !path.is_absolute() {
        return Err(Error::Development(format!(
            "--path must be absolute, got {:?}",
            request.path
        )));
    }
    // Existence is checked, but NOT the manifest: a directory that is
    // about to have an `objectiveai.json` written into it is a
    // perfectly reasonable thing to register, and the host reports a
    // missing or invalid manifest with its own error code when it
    // actually goes to build.
    match tokio::fs::metadata(&path).await {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(Error::Development(format!(
                "--path {:?} is not a directory",
                request.path
            )));
        }
        Err(e) => {
            return Err(Error::Development(format!("--path {:?}: {e}", request.path)));
        }
    }

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    let replaced = hubs.development_plugins.insert(key.clone(), path).is_some();

    Ok(Response {
        owner: key.0,
        name: key.1,
        version: key.2,
        path: request.path,
        replaced,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::create as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::create::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::development::plugins::mcp::create as sdk;
    use objectiveai_sdk::cli::command::development::plugins::mcp::create::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
