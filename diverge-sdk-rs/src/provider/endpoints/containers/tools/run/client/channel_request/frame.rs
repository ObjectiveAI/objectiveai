//! What a client's channel request frame carries for a tool container
//! run.

use std::error::Error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::mcp;
use crate::shared::mcp::session::Sessioned;
use crate::shared::containers::exec;
use crate::shared::containers::frontend;
use crate::shared::containers::terminal;
use crate::shared::containers::secrets::Values;
use crate::shared::containers::{fuse_mount, postgres, read, transfer, write_path};
use crate::shared::containers::daemon;

/// What a caller asks a provider for while a tool container runs.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Cancel`](Self::Cancel) |
/// | `1` | [`Filetree`](Self::Filetree) |
/// | `2` | [`Read`](Self::Read) |
/// | `3` | [`Write`](Self::Write) |
/// | `4` | [`Transfer`](Self::Transfer) |
/// | `5` | [`Postgres`](Self::Postgres) |
/// | `6` | unused |
/// | `7` | [`Daemon`](Self::Daemon) |
/// | `8` | [`McpListTools`](Self::McpListTools) |
/// | `9` | [`McpListResources`](Self::McpListResources) |
/// | `10` | [`McpCallTool`](Self::McpCallTool) |
/// | `11` | [`McpReadResource`](Self::McpReadResource) |
/// | `12` | [`McpNotifications`](Self::McpNotifications) |
/// | `13` | [`FuseMount`](Self::FuseMount) |
/// | `14` | [`Secrets`](Self::Secrets) |
/// | `15` | [`Exec`](Self::Exec) |
/// | `16` | [`TerminalOpen`](Self::TerminalOpen) |
/// | `17` | [`TerminalAttach`](Self::TerminalAttach) |
/// | `18` | [`TerminalStdin`](Self::TerminalStdin) |
/// | `19` | [`TerminalResize`](Self::TerminalResize) |
/// | `20` | [`TerminalClose`](Self::TerminalClose) |
/// | `21` | [`Frontend`](Self::Frontend) |
/// | `22` | [`McpCloseSession`](Self::McpCloseSession) |
///
/// The first seven tags are the same in every container scope — the
/// seventh unused in each, where an image's document was once asked
/// for and is now read on the image's own scope — in the same
/// order, so a reader of one is a reader of all; what follows is this
/// family's own exchange, and then the mount and the feed, which every
/// scope has again, each under a tag of its own. All of them but the
/// first reach INTO the container, which is the thing a caller cannot
/// dial: it runs on the provider. That is the whole reason these
/// channels open outward from the client rather than the other way.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Cancel the run: stop the container. Tag `0`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::wire::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. Finishing this one first would be a smaller way of
    /// saying the same thing, moments earlier.
    ///
    /// # What it adds over closing the connection
    ///
    /// The scope IS the container's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    Cancel,
    /// The container's filesystem, watched. Tag `1`.
    ///
    /// Carries nothing — the variant is bare — and the provider answers
    /// with a snapshot and then every change, for as long as the
    /// channel lives. See
    /// [`filetree`](crate::shared::containers::filetree).
    Filetree,
    /// One file, read out of the container. Tag `2`.
    ///
    /// See [`read`](crate::shared::containers::read) for why this is
    /// one file and never a directory.
    Read(read::request::Request),
    /// One file, written into the container. Tag `3`.
    ///
    /// Carries no content. The provider answers by opening a channel
    /// of its own asking for it — see
    /// [`write_path`](crate::shared::containers::write_path) for why
    /// it travels that direction, and
    /// [`write_bytes`](crate::shared::containers::write_bytes) for
    /// what comes back.
    Write(write_path::request::Request),
    /// One file, copied into another container. Tag `4`.
    ///
    /// The other container by its id, and the caller must be running
    /// it. The provider reads the file out of this
    /// container and writes it into that one on its own connections
    /// to the two proxies, and nothing of the file comes back here —
    /// one answer does. See
    /// [`transfer`](crate::shared::containers::transfer) for the rule.
    Transfer(transfer::request::Request),
    /// The caller's half of a database connection. Tag `5`.
    ///
    /// Opened once the caller has taken the provider's half, quoting
    /// the same connection; what comes back is everything the
    /// container wrote. See
    /// [`postgres`](crate::shared::containers::postgres) for the pair.
    Postgres(postgres::request::Postgres),
    /// A daemon connection the container's program opened, by the id
    /// the provider announced: this end's half of it. Tag `7`.
    ///
    /// What comes back on it is everything the PROGRAM sends, as
    /// [`daemon::client::Frame`](crate::shared::containers::daemon::client::Frame)s;
    /// its finish is the program's socket ended. See
    /// [`daemon`](crate::shared::containers::daemon) for the pair.
    Daemon(daemon::request::Daemon),
    /// What tools are there. Tag `8`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines, each [`Sessioned`]: the caller whose MCP session it
    /// belongs to, carried through to the container's proxy, as
    /// [`session`](crate::shared::mcp::session) states. See [`mcp::list_tools`](crate::shared::mcp::list_tools)
    /// for what it asks and what answers it.
    McpListTools(Sessioned<mcp::list_tools::request::Request>),
    /// What resources are there. Tag `9`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::list_resources`](crate::shared::mcp::list_resources) for
    /// what it asks and what answers it.
    McpListResources(Sessioned<mcp::list_resources::request::Request>),
    /// Run one tool. Tag `10`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See [`mcp::call_tool`](crate::shared::mcp::call_tool)
    /// for what it asks and what answers it.
    McpCallTool(Sessioned<mcp::call_tool::request::Request>),
    /// Read one resource. Tag `11`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::read_resource`](crate::shared::mcp::read_resource) for
    /// what it asks and what answers it.
    McpReadResource(Sessioned<mcp::read_resource::request::Request>),
    /// Everything the server says on its own account. Tag `12`.
    ///
    /// One of the five exchanges [`shared::mcp`](crate::shared::mcp)
    /// defines. See
    /// [`mcp::notifications`](crate::shared::mcp::notifications) for
    /// what it asks and what answers it.
    McpNotifications(Sessioned<mcp::notifications::request::Request>),
    /// A FUSE mount made while the container runs. Tag `13`.
    ///
    /// The caller serves one more file or directory into the
    /// container at a path, under an id of its own, as the mounts of
    /// the run's request are served. Answered once — by a
    /// [`fuse_mount::response::Frame`](crate::shared::containers::fuse_mount::response::Frame)
    /// saying the mount is made, or why not — and then the finish;
    /// the answer precedes every ask of the id. See
    /// [`fuse_mount`](crate::shared::containers::fuse_mount).
    FuseMount(fuse_mount::request::Request),
    /// The secrets' values, fed to the program at an activation. Tag
    /// `14`.
    ///
    /// Carries the [`Values`] as JSON — every name the program
    /// declared, each to its value, a `fixed` one read and a
    /// `rotating` one locked — which the provider relays to the proxy,
    /// and the proxy hands the program before it delivers an exchange.
    /// Opened by the caller when an idle container is about to
    /// receive an exchange and holds no values: after the
    /// registration's own feed, that is after every `Inactive`.
    /// Answered once — by a
    /// [`feed::response::Frame`](crate::shared::containers::secrets::feed::response::Frame),
    /// `Fed` or the program's refusal — and then the finish. See
    /// [`secrets`](crate::shared::containers::secrets).
    Secrets(Values),
    /// Run one command in the container, its output streamed: the
    /// provider opens an exec scope on the proxy and relays the stream,
    /// as [`exec`](crate::shared::containers::exec) states it. The
    /// caller abandoning the channel kills the command.
    Exec(exec::request::Request),
    /// Start a terminal the provider holds, answered by its id (see
    /// [`session`](crate::shared::containers::terminal::session)).
    TerminalOpen(terminal::open::Open),
    /// Stream a held terminal's output: its scrollback, then live.
    TerminalAttach(terminal::session::Id),
    /// Write bytes to a held terminal's stdin.
    TerminalStdin {
        /// The terminal.
        id: u32,
        /// What is written.
        bytes: Vec<u8>,
    },
    /// Set a held terminal's size.
    TerminalResize(terminal::session::Resize),
    /// Kill a held terminal's shell.
    TerminalClose(terminal::session::Id),
    /// One HTTP request to the program's frontend, its body asked for
    /// by its write id as a write's content is (see
    /// [`frontend`](crate::shared::containers::frontend)).
    Frontend(frontend::request::Request),
    /// The caller's MCP session, closed. Tag `22`.
    ///
    /// Not one of the five: the caller's side says it is done with the
    /// session it names. The rules:
    ///
    /// 1. The session closed is the one the [`Sessioned`] caller names,
    ///    by its [`key`](crate::shared::mcp::session::key); the request
    ///    carries nothing else.
    /// 2. The provider relays it to the container's proxy as it came,
    ///    and the proxy's answer back, as it does the five.
    /// 3. It is answered once — by a
    ///    [`close_session::response::Frame`](crate::shared::mcp::close_session::response::Frame),
    ///    `Closed` or the error — and then the finish; a session that is
    ///    not open is `Closed` all the same.
    /// 4. It is not the container's activity: it activates nothing and
    ///    feeds no secret.
    ///
    /// See [`mcp::close_session`](crate::shared::mcp::close_session).
    McpCloseSession(Sessioned<mcp::close_session::request::Request>),
}

/// Tag for [`Frame::Cancel`].
const CANCEL: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Tag for [`Frame::Read`].
const READ: u8 = 2;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 3;

/// Tag for [`Frame::Transfer`].
const TRANSFER: u8 = 4;

/// Tag for [`Frame::Postgres`].
const POSTGRES: u8 = 5;

/// Tag for [`Frame::Daemon`].
const DAEMON: u8 = 7;

/// Tag for [`Frame::McpListTools`].
const MCP_LIST_TOOLS: u8 = 8;

/// Tag for [`Frame::McpListResources`].
const MCP_LIST_RESOURCES: u8 = 9;

/// Tag for [`Frame::McpCallTool`].
const MCP_CALL_TOOL: u8 = 10;

/// Tag for [`Frame::McpReadResource`].
const MCP_READ_RESOURCE: u8 = 11;

/// Tag for [`Frame::McpNotifications`].
const MCP_NOTIFICATIONS: u8 = 12;

/// Tag for [`Frame::FuseMount`].
const FUSE_MOUNT: u8 = 13;

/// Tag for [`Frame::Secrets`].
const SECRETS: u8 = 14;

/// Tag for [`Frame::Exec`].
const EXEC: u8 = 15;

/// Tag for [`Frame::TerminalOpen`].
const TERMINAL_OPEN: u8 = 16;

/// Tag for [`Frame::TerminalAttach`].
const TERMINAL_ATTACH: u8 = 17;

/// Tag for [`Frame::TerminalStdin`].
const TERMINAL_STDIN: u8 = 18;

/// Tag for [`Frame::TerminalResize`].
const TERMINAL_RESIZE: u8 = 19;

/// Tag for [`Frame::TerminalClose`].
const TERMINAL_CLOSE: u8 = 20;

/// Tag for [`Frame::Frontend`].
const FRONTEND: u8 = 21;

/// Tag for [`Frame::McpCloseSession`].
const MCP_CLOSE_SESSION: u8 = 22;

impl Encode for Frame {
    /// The ordinary JSON failure, from whichever half has one.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Cancel => {
                out.extend_from_slice(&[CANCEL]);
                Ok(())
            }
            Frame::Filetree => {
                out.extend_from_slice(&[FILETREE]);
                Ok(())
            }
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Transfer(request) => {
                out.extend_from_slice(&[TRANSFER]);
                request.encode(out)
            }
            Frame::Postgres(request) => {
                out.extend_from_slice(&[POSTGRES]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::Daemon(request) => {
                out.extend_from_slice(&[DAEMON]);
                request.encode(out).map_err(|error| match error {})
            }
            Frame::McpListTools(request) => {
                out.extend_from_slice(&[MCP_LIST_TOOLS]);
                request.encode(out)
            }
            Frame::McpListResources(request) => {
                out.extend_from_slice(&[MCP_LIST_RESOURCES]);
                request.encode(out)
            }
            Frame::McpCallTool(request) => {
                out.extend_from_slice(&[MCP_CALL_TOOL]);
                request.encode(out)
            }
            Frame::McpReadResource(request) => {
                out.extend_from_slice(&[MCP_READ_RESOURCE]);
                request.encode(out)
            }
            Frame::McpNotifications(request) => {
                out.extend_from_slice(&[MCP_NOTIFICATIONS]);
                request.encode(out)
            }
            Frame::FuseMount(request) => {
                out.extend_from_slice(&[FUSE_MOUNT]);
                request.encode(out)
            }
            Frame::Secrets(values) => {
                out.extend_from_slice(&[SECRETS]);
                serde_json::to_writer(out, values)
            }
            Frame::Exec(request) => {
                out.extend_from_slice(&[EXEC]);
                request.encode(out)
            }
            Frame::TerminalOpen(size) => {
                out.extend_from_slice(&[TERMINAL_OPEN]);
                size.encode(out)
            }
            Frame::TerminalAttach(id) => {
                out.extend_from_slice(&[TERMINAL_ATTACH]);
                serde_json::to_writer(out, id)
            }
            Frame::TerminalStdin { id, bytes } => {
                out.extend_from_slice(&[TERMINAL_STDIN]);
                terminal::session::Stdin { id: *id, bytes }.encode(out).unwrap_or_else(|never| match never {});
                Ok(())
            }
            Frame::TerminalResize(resize) => {
                out.extend_from_slice(&[TERMINAL_RESIZE]);
                serde_json::to_writer(out, resize)
            }
            Frame::TerminalClose(id) => {
                out.extend_from_slice(&[TERMINAL_CLOSE]);
                serde_json::to_writer(out, id)
            }
            Frame::Frontend(request) => {
                out.extend_from_slice(&[FRONTEND]);
                serde_json::to_writer(out, request)
            }
            Frame::McpCloseSession(request) => {
                out.extend_from_slice(&[MCP_CLOSE_SESSION]);
                request.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Several ways to fail, and the parses among them name which.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CANCEL => Ok(Frame::Cancel),
            FILETREE => Ok(Frame::Filetree),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            TRANSFER => transfer::request::Request::decode(rest)
                .map(Frame::Transfer)
                .map_err(FrameError::Transfer),
            POSTGRES => postgres::request::Postgres::decode(rest)
                .map(Frame::Postgres)
                .map_err(FrameError::Postgres),
            DAEMON => daemon::request::Daemon::decode(rest)
                .map(Frame::Daemon)
                .map_err(FrameError::Daemon),
            MCP_LIST_TOOLS => Sessioned::<mcp::list_tools::request::Request>::decode(rest)
                .map(Frame::McpListTools)
                .map_err(FrameError::McpParams),
            MCP_LIST_RESOURCES => {
                Sessioned::<mcp::list_resources::request::Request>::decode(rest)
                    .map(Frame::McpListResources)
                    .map_err(FrameError::McpParams)
            }
            MCP_CALL_TOOL => Sessioned::<mcp::call_tool::request::Request>::decode(rest)
                .map(Frame::McpCallTool)
                .map_err(FrameError::McpParams),
            MCP_READ_RESOURCE => {
                Sessioned::<mcp::read_resource::request::Request>::decode(rest)
                    .map(Frame::McpReadResource)
                    .map_err(FrameError::McpParams)
            }
            MCP_NOTIFICATIONS => Sessioned::<mcp::notifications::request::Request>::decode(rest)
                .map(Frame::McpNotifications)
                .map_err(FrameError::McpParams),
            FUSE_MOUNT => fuse_mount::request::Request::decode(rest)
                .map(Frame::FuseMount)
                .map_err(FrameError::FuseMount),
            SECRETS => serde_json::from_slice(rest)
                .map(Frame::Secrets)
                .map_err(FrameError::Secrets),
            EXEC => exec::request::Request::decode(rest)
                .map(Frame::Exec)
                .map_err(FrameError::Exec),
            TERMINAL_OPEN => terminal::open::Open::decode(rest)
                .map(Frame::TerminalOpen)
                .map_err(FrameError::Terminal),
            TERMINAL_ATTACH => serde_json::from_slice(rest)
                .map(Frame::TerminalAttach)
                .map_err(FrameError::Terminal),
            TERMINAL_STDIN => terminal::session::Stdin::decode(rest)
                .map(|stdin| Frame::TerminalStdin { id: stdin.id, bytes: stdin.bytes.to_vec() })
                .map_err(|_| FrameError::TerminalStdin),
            TERMINAL_RESIZE => serde_json::from_slice(rest)
                .map(Frame::TerminalResize)
                .map_err(FrameError::Terminal),
            TERMINAL_CLOSE => serde_json::from_slice(rest)
                .map(Frame::TerminalClose)
                .map_err(FrameError::Terminal),
            FRONTEND => serde_json::from_slice(rest)
                .map(Frame::Frontend)
                .map_err(FrameError::Frontend),
            MCP_CLOSE_SESSION => Sessioned::<mcp::close_session::request::Request>::decode(rest)
                .map(Frame::McpCloseSession)
                .map_err(FrameError::McpParams),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tool container run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's.
    UnknownTag(u8),
    /// The read request did not parse.
    Read(serde_json::Error),
    /// The write request did not parse.
    Write(serde_json::Error),
    /// The transfer request did not parse.
    Transfer(serde_json::Error),
    /// The connection id was not four bytes.
    Postgres(postgres::request::PostgresError),
    /// The daemon connection's id was not four bytes.
    Daemon(daemon::request::DaemonError),
    /// One of the five MCP exchanges' params, or a close's session,
    /// did not parse.
    ///
    /// One variant for six tags, because they fail the same way and
    /// the tag already said which was meant. Naming each would be five
    /// cases every reader matches and none of them distinguishes
    /// anything a caller could act on.
    McpParams(serde_json::Error),
    /// The mount request did not parse.
    FuseMount(serde_json::Error),
    /// The secrets' values did not parse as JSON. The message is the
    /// parser's, which quotes no value.
    Secrets(serde_json::Error),
    /// The exec request did not parse.
    Exec(serde_json::Error),
    /// A terminal request did not parse.
    Terminal(serde_json::Error),
    /// A terminal's stdin held no id.
    TerminalStdin,
    /// The frontend request did not parse.
    Frontend(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown tools run channel request tag {tag}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
            }
            FrameError::Transfer(error) => {
                write!(f, "transfer request did not parse: {error}")
            }
            FrameError::Postgres(error) => write!(f, "{error}"),
            FrameError::Daemon(error) => write!(f, "{error}"),
            FrameError::McpParams(error) => {
                write!(f, "mcp request params did not parse: {error}")
            }
            FrameError::FuseMount(error) => {
                write!(f, "fuse mount request did not parse: {error}")
            }
            FrameError::Secrets(error) => {
                write!(f, "secrets feed did not parse: {error}")
            }
            FrameError::Exec(error) => {
                write!(f, "exec request did not parse: {error}")
            }
            FrameError::Terminal(error) => {
                write!(f, "terminal request did not parse: {error}")
            }
            FrameError::TerminalStdin => f.write_str("terminal stdin held no id"),
            FrameError::Frontend(error) => {
                write!(f, "frontend request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::Transfer(error)
            | FrameError::McpParams(error)
            | FrameError::FuseMount(error)
            | FrameError::Secrets(error)
            | FrameError::Exec(error)
            | FrameError::Terminal(error)
            | FrameError::Frontend(error) => Some(error),
            FrameError::Postgres(error) => Some(error),
            FrameError::Daemon(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) | FrameError::TerminalStdin => None,
        }
    }
}
