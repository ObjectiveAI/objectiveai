//! What answers the seams in a build without the stand-in, until the wire
//! does: nothing. Every list is empty, because there is nothing to list;
//! everything else says the network part isn't there yet. No invented
//! agents, machines, rooms or people.

// The stand-in answers instead in a build with it; tests use this either way.
#![cfg_attr(feature = "stand-in", allow(dead_code))]

use async_trait::async_trait;
use futures::stream;
use rmcp::model::{CallToolRequestParams, CallToolResult, ErrorData, ListToolsResult, ReadResourceResult, ServerNotification};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::{agents, tools};
use diverge_sdk::provider::endpoints::volumes;
use diverge_sdk::shared::error::Error as WireError;
use diverge_sdk::shared::filetree::response::Node;

use crate::daemon::{Daemon, Frames, NewProvider, ProviderEntry};
use crate::machines::{Machines, ReadFrame};
use crate::spaces::{Answer, Container, HostCall, Id, Invite, Joined, Knock, Knocking, SpaceEntry, Spaces};

/// What every verb answers. The same words are the screen's, in `src/strings.ts`.
pub const NOT_YET: &str = "This build has no network part yet: no daemon, no machines, no rooms. Nothing here reaches anyone.";

fn not_yet() -> WireError {
    WireError(json!({ "message": NOT_YET }))
}

fn not_yet_mcp() -> ErrorData {
    ErrorData::internal_error(NOT_YET, None)
}

/// No daemon, no machines and no rooms.
pub struct Absent;

#[async_trait]
impl Daemon for Absent {
    async fn agents_create(&self, _: agents::create::client::request::Frame) -> agents::create::server::response::Frame {
        agents::create::server::response::Frame::Error(not_yet())
    }

    async fn agents_delete(&self, _: agents::delete::client::request::Frame) -> agents::delete::server::response::Frame {
        agents::delete::server::response::Frame::Error(not_yet())
    }

    async fn agents_message(&self, _: agents::message::client::request::Frame, _: CancellationToken) -> agents::message::server::response::Frame {
        agents::message::server::response::Frame::Error(not_yet())
    }

    fn agents_logs(&self, _: agents::logs::client::request::Frame, _: CancellationToken) -> Frames<agents::logs::server::response::Frame> {
        Box::pin(stream::iter([agents::logs::server::response::Frame::Error(not_yet())]))
    }

    fn agents_list(&self, _: agents::list::client::request::Frame) -> Frames<agents::list::server::response::Frame> {
        Box::pin(stream::empty())
    }

    async fn agents_edit(&self, _: agents::edit::client::request::Frame) -> agents::edit::server::response::Frame {
        agents::edit::server::response::Frame::Error(not_yet())
    }

    async fn tools_create(&self, _: tools::create::client::request::Frame) -> tools::create::server::response::Frame {
        tools::create::server::response::Frame::Error(not_yet())
    }

    async fn tools_edit(&self, _: tools::edit::client::request::Frame) -> tools::edit::server::response::Frame {
        tools::edit::server::response::Frame::Error(not_yet())
    }

    async fn tools_connect(&self, _: tools::connect::client::request::Frame) -> tools::connect::server::response::Frame {
        tools::connect::server::response::Frame::Error(not_yet())
    }

    async fn tools_attach(&self, _: tools::attach::client::request::Frame) -> tools::attach::server::response::Frame {
        tools::attach::server::response::Frame::Error(not_yet())
    }

    async fn tools_detach(&self, _: tools::detach::client::request::Frame) -> tools::detach::server::response::Frame {
        tools::detach::server::response::Frame::Error(not_yet())
    }

    async fn tools_delete(&self, _: tools::delete::client::request::Frame) -> tools::delete::server::response::Frame {
        tools::delete::server::response::Frame::Error(not_yet())
    }

    fn tools_list(&self, _: tools::list::client::request::Frame) -> Frames<tools::list::server::response::Frame> {
        Box::pin(stream::empty())
    }

    async fn providers_list(&self) -> Vec<ProviderEntry> {
        Vec::new()
    }

    async fn providers_add(&self, _: NewProvider) -> Result<ProviderEntry, String> {
        Err(NOT_YET.into())
    }

    async fn providers_remove(&self, _: Identity) -> Result<(), String> {
        Err(NOT_YET.into())
    }
}

#[async_trait]
impl Machines for Absent {
    async fn volumes_list(&self, _: &Identity) -> volumes::list::server::response::Frame {
        volumes::list::server::response::Frame::Error(not_yet())
    }

    async fn volumes_stat(&self, _: &Identity, _: volumes::stat::client::request::Frame) -> volumes::stat::server::response::Frame {
        volumes::stat::server::response::Frame::Error(not_yet())
    }

    fn volumes_read(&self, _: &Identity, _: volumes::read::client::request::Frame) -> Frames<ReadFrame> {
        Box::pin(stream::iter([ReadFrame::Error(not_yet())]))
    }

    async fn volumes_write(&self, _: &Identity, _: volumes::write::client::request::Frame, _: Vec<u8>) -> volumes::write::server::response::Frame {
        volumes::write::server::response::Frame::Error(not_yet())
    }

    async fn volumes_filetree(&self, _: &Identity, _: volumes::filetree::client::request::Frame) -> volumes::filetree::server::response::Frame {
        volumes::filetree::server::response::Frame::Error(not_yet())
    }

    async fn volumes_create_capacity(&self, _: &Identity) -> volumes::create_capacity::server::response::Frame {
        volumes::create_capacity::server::response::Frame::Error(not_yet())
    }

    async fn volumes_create(&self, _: &Identity, _: volumes::create::client::request::Frame) -> volumes::create::server::response::Frame {
        volumes::create::server::response::Frame::Error(not_yet())
    }

    async fn volumes_edit_capacity(&self, _: &Identity, _: volumes::edit_capacity::client::request::Frame) -> volumes::edit_capacity::server::response::Frame {
        volumes::edit_capacity::server::response::Frame::Error(not_yet())
    }

    async fn volumes_edit(&self, _: &Identity, _: volumes::edit::client::request::Frame) -> volumes::edit::server::response::Frame {
        volumes::edit::server::response::Frame::Error(not_yet())
    }

    async fn volumes_delete(&self, _: &Identity, _: volumes::delete::client::request::Frame) -> volumes::delete::server::response::Frame {
        volumes::delete::server::response::Frame::Error(not_yet())
    }
}

#[async_trait]
impl Spaces for Absent {
    async fn list(&self) -> Vec<SpaceEntry> {
        Vec::new()
    }

    async fn host(&self, _: Container) -> Result<Id, WireError> {
        Err(not_yet())
    }

    fn knocks(&self, _: CancellationToken) -> Frames<Knock> {
        Box::pin(stream::empty())
    }

    async fn answer(&self, _: u64, _: Answer) -> Result<Knock, String> {
        Err(NOT_YET.into())
    }

    async fn pending(&self, _: u64) -> Option<Knock> {
        None
    }

    async fn join(&self, _: &Invite, _: &Knocking) -> Joined {
        Joined::Error(not_yet())
    }

    async fn leave(&self, _: &Id) -> Result<(), String> {
        Err(NOT_YET.into())
    }

    async fn tools(&self, _: &Id) -> Result<ListToolsResult, ErrorData> {
        Err(not_yet_mcp())
    }

    async fn read(&self, _: &Id, _: &str) -> Result<ReadResourceResult, ErrorData> {
        Err(not_yet_mcp())
    }

    async fn call(&self, _: &Id, _: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        Err(not_yet_mcp())
    }

    fn notifications(&self, _: &Id, _: CancellationToken) -> Frames<ServerNotification> {
        Box::pin(stream::empty())
    }

    async fn invite(&self, _: &Id) -> Option<Invite> {
        None
    }

    async fn home(&self) -> Option<Id> {
        None
    }

    async fn profile(&self) -> Option<Id> {
        None
    }

    fn host_calls(&self, _: CancellationToken) -> Frames<HostCall> {
        Box::pin(stream::empty())
    }

    async fn table_tree(&self, _: &Id) -> Result<Vec<Node>, WireError> {
        Err(not_yet())
    }

    async fn table_read(&self, _: &Id, _: &[String]) -> Result<Vec<u8>, WireError> {
        Err(not_yet())
    }

    async fn table_write(&self, _: &Id, _: &[String], _: Vec<u8>) -> Result<(), WireError> {
        Err(not_yet())
    }

    async fn transfer(&self, _: &Id, _: &[String], _: &Id) -> Result<(), WireError> {
        Err(not_yet())
    }

    async fn restart(&self, _: &Id) -> Result<(), WireError> {
        Err(not_yet())
    }
}

#[cfg(test)]
mod tests {
    use super::NOT_YET;

    #[test]
    fn its_words_are_the_screens_words() {
        assert!(include_str!("../../src/strings.ts").contains(NOT_YET), "src/strings.ts holds these words");
    }
}
