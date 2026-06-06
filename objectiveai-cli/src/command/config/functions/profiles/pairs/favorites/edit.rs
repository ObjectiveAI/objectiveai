//! `config functions profiles pairs favorites edit` — mutate a named
//! entry in the function-profiles pair-favorites list. Supports
//! updating the note and the commit-on-path independently for each of
//! the function and profile sides (set or remove); drop and re-add to
//! change remote/owner/repo.

use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::edit::{
    Request, RequestCommitChange, Response,
};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let favorite = config
        .functions()
        .profiles()
        .pairs()
        .edit_favorite(&request.name)?;
    if let Some(note) = request.note {
        favorite.set_note(note)?;
    }
    if let Some(change) = request.function_commit {
        apply_commit_change(&mut favorite.function, change);
    }
    if let Some(change) = request.profile_commit {
        apply_commit_change(&mut favorite.profile, change);
    }
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

fn apply_commit_change(path: &mut RemotePathCommitOptional, change: RequestCommitChange) {
    let new_commit = match change {
        RequestCommitChange::Set(c) => Some(c),
        RequestCommitChange::Remove => None,
    };
    match path {
        RemotePathCommitOptional::Github { commit, .. } => *commit = new_commit,
        RemotePathCommitOptional::Filesystem { commit, .. } => *commit = new_commit,
        RemotePathCommitOptional::Mock { .. } => {}
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::edit as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::edit::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::edit as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::edit::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
