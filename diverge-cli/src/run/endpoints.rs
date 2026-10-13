//! Every request of the daemon's wire, put by its name.
//!
//! One arm per request, so that a request added to the SDK is one
//! this cannot compile without — the same rule the daemon's dispatch
//! keeps. Each arm names its [`shape`] and the
//! endpoint's own request and answer types; the four downloads and
//! the four uploads implement the shapes' [`Download`] and [`Upload`]
//! beside the arms, since their executors are not one function.

use std::path::Path;

use bytes::Bytes;
use diverge_sdk::daemon::client::stream::{ExecuteStream, OpenError};
use diverge_sdk::daemon::endpoints::{accounts, agents, dependency_tools, postgres, providers, roles, standalone_tools, volumes};
use diverge_sdk::wire::client::handle::Handle;
use serde_json::Value;

use super::shape::{self, Content, Download, Downloaded, Upload};
use super::{Emitter, Error, Request};

/// Put `request` on `handle` and print its answers; `local_path` is
/// the local file or directory the four uploads read from and the
/// four downloads write into, which they need and every other
/// request refuses.
pub async fn run(request: &Request, handle: &Handle, local_path: Option<&Path>, emitter: &mut Emitter) -> Result<(), Error> {
    let path = request.path.as_str();
    if local_path.is_some() && !takes_local(path) {
        return Err(Error::Path {
            path: request.path.clone(),
            given: true,
        });
    }
    match path {
        "accounts/create" => shape::one_shot::<accounts::create::client::request::Frame, accounts::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "accounts/delete" => shape::one_shot::<accounts::delete::client::request::Frame, accounts::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "accounts/edit" => shape::one_shot::<accounts::edit::client::request::Frame, accounts::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "accounts/get" => shape::one_shot::<accounts::get::client::request::Frame, accounts::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "accounts/list" => shape::stream::<accounts::list::client::request::Frame, accounts::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "accounts/tag" => shape::one_shot::<accounts::tag::client::request::Frame, accounts::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "accounts/untag" => shape::one_shot::<accounts::untag::client::request::Frame, accounts::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/create" => shape::one_shot::<agents::create::client::request::Frame, agents::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/delete" => shape::one_shot::<agents::delete::client::request::Frame, agents::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/download" => shape::download::<agents::download::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "agents/edit" => shape::one_shot::<agents::edit::client::request::Frame, agents::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/filetree" => shape::stream::<agents::filetree::client::request::Frame, agents::filetree::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_snapshot).await,
        "agents/get" => shape::one_shot::<agents::get::client::request::Frame, agents::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/list" => shape::stream::<agents::list::client::request::Frame, agents::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "agents/logs" => shape::stream::<agents::logs::client::request::Frame, agents::logs::server::response::Frame, _>(handle, &request.bytes, emitter, shape::never).await,
        "agents/message" => shape::pending::<agents::message::client::request::Frame, agents::message::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/tag" => shape::one_shot::<agents::tag::client::request::Frame, agents::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/templates/create" => shape::one_shot::<agents::templates::create::client::request::Frame, agents::templates::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/templates/delete" => shape::one_shot::<agents::templates::delete::client::request::Frame, agents::templates::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/templates/get" => shape::one_shot::<agents::templates::get::client::request::Frame, agents::templates::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/templates/list" => shape::stream::<agents::templates::list::client::request::Frame, agents::templates::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "agents/templates/tag" => shape::one_shot::<agents::templates::tag::client::request::Frame, agents::templates::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/templates/untag" => shape::one_shot::<agents::templates::untag::client::request::Frame, agents::templates::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/transfer" => shape::one_shot::<agents::transfer::client::request::Frame, agents::transfer::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/untag" => shape::one_shot::<agents::untag::client::request::Frame, agents::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "agents/upload" => shape::upload::<agents::upload::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "dependency_tools/delete" => shape::one_shot::<dependency_tools::delete::client::request::Frame, dependency_tools::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "dependency_tools/download" => shape::download::<dependency_tools::download::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "dependency_tools/filetree" => shape::stream::<dependency_tools::filetree::client::request::Frame, dependency_tools::filetree::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_snapshot).await,
        "dependency_tools/get" => shape::one_shot::<dependency_tools::get::client::request::Frame, dependency_tools::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "dependency_tools/list" => shape::stream::<dependency_tools::list::client::request::Frame, dependency_tools::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "dependency_tools/tag" => shape::one_shot::<dependency_tools::tag::client::request::Frame, dependency_tools::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "dependency_tools/transfer" => shape::one_shot::<dependency_tools::transfer::client::request::Frame, dependency_tools::transfer::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "dependency_tools/untag" => shape::one_shot::<dependency_tools::untag::client::request::Frame, dependency_tools::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "dependency_tools/upload" => shape::upload::<dependency_tools::upload::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "postgres/get" => shape::one_shot::<postgres::get::client::request::Frame, postgres::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "postgres/list" => shape::stream::<postgres::list::client::request::Frame, postgres::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "providers/daemons/add" => shape::one_shot::<providers::daemons::add::client::request::Frame, providers::daemons::add::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/daemons/delete" => shape::one_shot::<providers::daemons::delete::client::request::Frame, providers::daemons::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/daemons/edit" => shape::one_shot::<providers::daemons::edit::client::request::Frame, providers::daemons::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/daemons/get" => shape::one_shot::<providers::daemons::get::client::request::Frame, providers::daemons::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/daemons/list" => shape::stream::<providers::daemons::list::client::request::Frame, providers::daemons::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "providers/daemons/tag" => shape::one_shot::<providers::daemons::tag::client::request::Frame, providers::daemons::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/daemons/untag" => shape::one_shot::<providers::daemons::untag::client::request::Frame, providers::daemons::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/add" => shape::one_shot::<providers::incoming::add::client::request::Frame, providers::incoming::add::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/delete" => shape::one_shot::<providers::incoming::delete::client::request::Frame, providers::incoming::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/edit" => shape::one_shot::<providers::incoming::edit::client::request::Frame, providers::incoming::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/get" => shape::one_shot::<providers::incoming::get::client::request::Frame, providers::incoming::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/list" => shape::stream::<providers::incoming::list::client::request::Frame, providers::incoming::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "providers/incoming/tag" => shape::one_shot::<providers::incoming::tag::client::request::Frame, providers::incoming::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/incoming/untag" => shape::one_shot::<providers::incoming::untag::client::request::Frame, providers::incoming::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/add" => shape::one_shot::<providers::outgoing::add::client::request::Frame, providers::outgoing::add::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/delete" => shape::one_shot::<providers::outgoing::delete::client::request::Frame, providers::outgoing::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/edit" => shape::one_shot::<providers::outgoing::edit::client::request::Frame, providers::outgoing::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/get" => shape::one_shot::<providers::outgoing::get::client::request::Frame, providers::outgoing::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/list" => shape::stream::<providers::outgoing::list::client::request::Frame, providers::outgoing::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "providers/outgoing/tag" => shape::one_shot::<providers::outgoing::tag::client::request::Frame, providers::outgoing::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "providers/outgoing/untag" => shape::one_shot::<providers::outgoing::untag::client::request::Frame, providers::outgoing::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/create" => shape::one_shot::<roles::create::client::request::Frame, roles::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/delete" => shape::one_shot::<roles::delete::client::request::Frame, roles::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/edit" => shape::one_shot::<roles::edit::client::request::Frame, roles::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/get" => shape::one_shot::<roles::get::client::request::Frame, roles::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/list" => shape::stream::<roles::list::client::request::Frame, roles::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "roles/tag" => shape::one_shot::<roles::tag::client::request::Frame, roles::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "roles/untag" => shape::one_shot::<roles::untag::client::request::Frame, roles::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/attach" => shape::one_shot::<standalone_tools::attach::client::request::Frame, standalone_tools::attach::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/connect" => Err(Error::Unsupported(request.path.clone())),
        "standalone_tools/create" => shape::one_shot::<standalone_tools::create::client::request::Frame, standalone_tools::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/delete" => shape::one_shot::<standalone_tools::delete::client::request::Frame, standalone_tools::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/detach" => shape::one_shot::<standalone_tools::detach::client::request::Frame, standalone_tools::detach::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/download" => shape::download::<standalone_tools::download::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "standalone_tools/edit" => shape::one_shot::<standalone_tools::edit::client::request::Frame, standalone_tools::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/filetree" => shape::stream::<standalone_tools::filetree::client::request::Frame, standalone_tools::filetree::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_snapshot).await,
        "standalone_tools/get" => shape::one_shot::<standalone_tools::get::client::request::Frame, standalone_tools::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/list" => shape::stream::<standalone_tools::list::client::request::Frame, standalone_tools::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "standalone_tools/register" => shape::one_shot::<standalone_tools::register::client::request::Frame, standalone_tools::register::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/tag" => shape::one_shot::<standalone_tools::tag::client::request::Frame, standalone_tools::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/templates/create" => shape::one_shot::<standalone_tools::templates::create::client::request::Frame, standalone_tools::templates::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/templates/delete" => shape::one_shot::<standalone_tools::templates::delete::client::request::Frame, standalone_tools::templates::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/templates/get" => shape::one_shot::<standalone_tools::templates::get::client::request::Frame, standalone_tools::templates::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/templates/list" => shape::stream::<standalone_tools::templates::list::client::request::Frame, standalone_tools::templates::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "standalone_tools/templates/tag" => shape::one_shot::<standalone_tools::templates::tag::client::request::Frame, standalone_tools::templates::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/templates/untag" => shape::one_shot::<standalone_tools::templates::untag::client::request::Frame, standalone_tools::templates::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/transfer" => shape::one_shot::<standalone_tools::transfer::client::request::Frame, standalone_tools::transfer::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/untag" => shape::one_shot::<standalone_tools::untag::client::request::Frame, standalone_tools::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "standalone_tools/upload" => shape::upload::<standalone_tools::upload::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "volumes/create" => shape::one_shot::<volumes::create::client::request::Frame, volumes::create::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/delete" => shape::one_shot::<volumes::delete::client::request::Frame, volumes::delete::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/download" => shape::download::<volumes::download::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        "volumes/edit" => shape::one_shot::<volumes::edit::client::request::Frame, volumes::edit::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/filetree" => shape::one_shot::<volumes::filetree::client::request::Frame, volumes::filetree::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/get" => shape::one_shot::<volumes::get::client::request::Frame, volumes::get::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/list" => shape::stream::<volumes::list::client::request::Frame, volumes::list::server::response::Frame, _>(handle, &request.bytes, emitter, shape::at_listed).await,
        "volumes/stat" => shape::one_shot::<volumes::stat::client::request::Frame, volumes::stat::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/tag" => shape::one_shot::<volumes::tag::client::request::Frame, volumes::tag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/transfer" => shape::one_shot::<volumes::transfer::client::request::Frame, volumes::transfer::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/untag" => shape::one_shot::<volumes::untag::client::request::Frame, volumes::untag::server::response::Frame, _>(handle, &request.bytes, emitter).await,
        "volumes/upload" => shape::upload::<volumes::upload::client::request::Frame>(handle, &request.bytes, local(local_path, &request.path)?, emitter).await,
        _ => Err(Error::NoSuchRequest(request.path.clone())),
    }
}

/// Whether the request reads or writes local files.
fn takes_local(path: &str) -> bool {
    path.ends_with("/download") || path.ends_with("/upload")
}

/// The local path a file request was given, or the refusal for one
/// given none.
fn local<'a>(local_path: Option<&'a Path>, path: &str) -> Result<&'a Path, Error> {
    local_path.ok_or_else(|| Error::Path {
        path: path.to_string(),
        given: false,
    })
}

impl Download for agents::download::client::request::Frame {
    type Item = agents::download::client::execute::Item;
    type Failure = agents::download::server::response::FrameError;

    async fn execute(&self, handle: &Handle) -> Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>> {
        agents::download::client::execute::execute(handle, self).await
    }
}

impl Downloaded for agents::download::client::execute::Item {
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self> {
        match self {
            agents::download::client::execute::Item::Chunk { path, body } => Ok((path, body)),
            other => Err(other),
        }
    }
}


impl Download for dependency_tools::download::client::request::Frame {
    type Item = dependency_tools::download::client::execute::Item;
    type Failure = dependency_tools::download::server::response::FrameError;

    async fn execute(&self, handle: &Handle) -> Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>> {
        dependency_tools::download::client::execute::execute(handle, self).await
    }
}

impl Downloaded for dependency_tools::download::client::execute::Item {
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self> {
        match self {
            dependency_tools::download::client::execute::Item::Chunk { path, body } => Ok((path, body)),
            other => Err(other),
        }
    }
}


impl Download for standalone_tools::download::client::request::Frame {
    type Item = standalone_tools::download::client::execute::Item;
    type Failure = standalone_tools::download::server::response::FrameError;

    async fn execute(&self, handle: &Handle) -> Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>> {
        standalone_tools::download::client::execute::execute(handle, self).await
    }
}

impl Downloaded for standalone_tools::download::client::execute::Item {
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self> {
        match self {
            standalone_tools::download::client::execute::Item::Chunk { path, body } => Ok((path, body)),
            other => Err(other),
        }
    }
}


impl Download for volumes::download::client::request::Frame {
    type Item = volumes::download::client::execute::Item;
    type Failure = volumes::download::server::response::FrameError;

    async fn execute(&self, handle: &Handle) -> Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>> {
        volumes::download::client::execute::execute(handle, self).await
    }
}

impl Downloaded for volumes::download::client::execute::Item {
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self> {
        match self {
            volumes::download::client::execute::Item::Chunk { path, body } => Ok((path, body)),
            other => Err(other),
        }
    }
}

impl Upload for agents::upload::client::request::Frame {
    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<Value, String>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        let answer = agents::upload::client::execute::execute(handle, self, source).await.map_err(|error| error.to_string())?;
        serde_json::to_value(&answer).map_err(|error| error.to_string())
    }
}


impl Upload for dependency_tools::upload::client::request::Frame {
    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<Value, String>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        let answer = dependency_tools::upload::client::execute::execute(handle, self, source).await.map_err(|error| error.to_string())?;
        serde_json::to_value(&answer).map_err(|error| error.to_string())
    }
}


impl Upload for standalone_tools::upload::client::request::Frame {
    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<Value, String>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        let answer = standalone_tools::upload::client::execute::execute(handle, self, source).await.map_err(|error| error.to_string())?;
        serde_json::to_value(&answer).map_err(|error| error.to_string())
    }
}


impl Upload for volumes::upload::client::request::Frame {
    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<Value, String>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        let answer = volumes::upload::client::execute::execute(handle, self, source).await.map_err(|error| error.to_string())?;
        serde_json::to_value(&answer).map_err(|error| error.to_string())
    }
}
