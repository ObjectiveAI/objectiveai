//! This scope's frames, named for the machinery.

use std::sync::Arc;

use bytes::Bytes;

use super::super::super::client::channel_request;
use super::super::super::client::channel_response::write_bytes;
use super::super::channel_response::{filetree, read, write_path};
use super::super::{channel_request as ask, response};
use crate::decode::Decode as _;
use crate::endpoints::containers::server::family::{Family, Opened};
use crate::endpoints::containers::server::run::Run;
use crate::endpoints::containers::server::serve::tool;
use crate::endpoints::containers::server::{encoded::encoded, render};
use crate::shared;
use crate::shared::error::Error;
use crate::shared::filetree as tree;

/// A tool container connection: the five MCP exchanges are the
/// family's own, and the one ask this end makes is a write's content.
pub(crate) struct Connect;

impl Family for Connect {
    type Request = channel_request::Frame;
    type Exchange = tool::Exchange;

    fn classify(request: channel_request::Frame) -> Opened<tool::Exchange> {
        match request {
            channel_request::Frame::Disconnect => Opened::Stop,
            channel_request::Frame::Filetree => Opened::Filetree,
            channel_request::Frame::Read(request) => Opened::Read(request.path),
            channel_request::Frame::Write(request) => Opened::Write {
                write_id: request.write_id,
                path: request.path,
            },
            channel_request::Frame::Postgres(request) => Opened::Postgres(request.connection_id),
            channel_request::Frame::McpListTools(request) => Opened::Exchange(tool::Exchange::ListTools(request)),
            channel_request::Frame::McpListResources(request) => {
                Opened::Exchange(tool::Exchange::ListResources(request))
            }
            channel_request::Frame::McpCallTool(request) => Opened::Exchange(tool::Exchange::CallTool(request)),
            channel_request::Frame::McpReadResource(request) => Opened::Exchange(tool::Exchange::ReadResource(request)),
            channel_request::Frame::McpNotifications(_) => Opened::Exchange(tool::Exchange::Notifications),
        }
    }

    fn serve(run: Arc<Run>, channel: u32, exchange: tool::Exchange) -> impl Future<Output = ()> + Send + 'static {
        tool::serve(run, channel, exchange)
    }

    fn error(error: &Error) -> Option<Vec<u8>> {
        encoded(&response::Frame(error.clone()))
    }

    fn write_ask(write_id: u32) -> Option<Vec<u8>> {
        encoded(&ask::Frame(shared::containers::write_bytes::request::Request { write_id }))
    }

    fn content(payload: &Bytes) -> Result<Bytes, Error> {
        match write_bytes::Frame::decode(payload) {
            Ok(write_bytes::Frame::Body(body)) => Ok(payload.slice_ref(body.0)),
            Ok(write_bytes::Frame::Error(error)) => Err(error),
            Err(error) => Err(render::proxy(error)),
        }
    }

    fn filetree(frame: tree::response::Frame) -> Option<Vec<u8>> {
        encoded(&filetree::Frame::Filetree(frame))
    }

    fn filetree_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&filetree::Frame::Error(error.clone()))
    }

    fn read_body(bytes: &[u8]) -> Option<Vec<u8>> {
        encoded(&read::Frame::Body(shared::containers::read::response::Frame(bytes)))
    }

    fn read_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&read::Frame::Error(error.clone()))
    }

    fn written() -> Option<Vec<u8>> {
        encoded(&write_path::Frame::Written(shared::containers::write_path::response::Frame))
    }

    fn write_error(error: &Error) -> Option<Vec<u8>> {
        encoded(&write_path::Frame::Error(error.clone()))
    }
}
