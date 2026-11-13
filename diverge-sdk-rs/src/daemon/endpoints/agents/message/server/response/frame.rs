//! What a server's response frame carries for a message.

use serde::Serialize;
use std::fmt;

use crate::daemon::agent_log::ItemWrapper;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::info::Lacking;
use crate::shared::error::Error;

/// A message's answer: its fate, or — watched — what the agent's log
/// took of it.
///
/// A payload leads with one byte saying which — `0` for
/// [`Delivered`](Self::Delivered), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error), `3`
/// for [`NoChat`](Self::NoChat), `4` for
/// [`UnsupportedModalities`](Self::UnsupportedModalities), `5` for
/// [`NoStructuredDecisions`](Self::NoStructuredDecisions), `6` for
/// [`Item`](Self::Item), `7` for [`Dropped`](Self::Dropped) — and only
/// the error, the modalities and the item carry anything after it.
///
/// # Unwatched
///
/// One answer per scope, before the finish that ends it — but the
/// reply comes when the fate is known, which may be long after the
/// ask: a message for a sleeping agent waits for its run to start,
/// and a structured decision waits for everything the agent was
/// handed before it to finish; nothing times either out.
/// [`Delivered`](Self::Delivered) is the agent's program having it.
/// The agent's answers to a structured decision are not this answer:
/// they are read in its log, or by watching.
///
/// # Watched
///
/// A request whose `watch` is `true` is answered with the part of the
/// agent's log that is the request's, each item one
/// [`Item`](Self::Item), exactly as a
/// [`logs`](crate::daemon::endpoints::logs) read of the agent gives
/// it, in the log's order. The rules:
///
/// 1. **Nothing before the echo.** No [`Delivered`](Self::Delivered)
///    is sent, and no item the log took before the request's echo.
/// 2. **The echo is first.** For a chat message, the first user part
///    the log kept of it — an item whose `type` is one of the five
///    `user_*` content kinds; for a structured decision, its
///    `user_structured_decisions` item. The agent's program says it
///    as it takes the request up.
/// 3. **Then every item the log takes after it**, whatever it is of —
///    the rest of the request's user parts, what the agent says, its
///    tool calls, another sender's message that joined the same run,
///    an error — up to and including the last, which is:
///    - for a chat message, the first `inactive` after the echo: the
///      agent ceased working, the end of the run the echo landed in.
///      A container that goes down under the run keeps `inactive`
///      before its `undeployed`, and the `inactive` is the last;
///    - for a structured decision, the
///      `structured_decisions_response` whose `id` is the echoed
///      decision's, where one comes; else the first `inactive`, as for
///      a chat message. Which it was is the last item's `type`.
/// 4. **Then the finish.** A cancel finishes the scope sooner, and so
///    does the agent's deletion; the items sent precede the finish.
/// 5. **A request that reaches no echo has one answer**, with nothing
///    before it, then the finish: any of the unwatched answers save
///    [`Delivered`](Self::Delivered) — the request was refused, or
///    failed on its way — or [`Dropped`](Self::Dropped).
///
/// # Dropped
///
/// [`Dropped`](Self::Dropped) is the answer to a watched request the
/// agent's program accepted and never echoed: the run it was
/// delivered into is over — or went on, through the agent ceasing
/// work twice since the delivery — and the log kept nothing of it.
/// It will not be echoed later: a request belongs to the one run it
/// was delivered into, and the daemon no longer knows it by then.
/// The wait a request makes before it is delivered — for a run to
/// start, for a structured decision ahead of it — counts for nothing
/// here: the runs that come and go meanwhile are not its own.
///
/// # What the agent's image says
///
/// The daemon holds the request to the agent's image's `/DIVERGE.md`,
/// as it has it on record, before anything is sent on:
/// [`NoChat`](Self::NoChat) or
/// [`NoStructuredDecisions`](Self::NoStructuredDecisions) where the
/// image states `false` to the request's kind, and
/// [`UnsupportedModalities`](Self::UnsupportedModalities) where the
/// request carries what the image does not take. Either way the
/// request reached no provider and no container, nothing was started
/// for it, and the agent's log has no line of it.
///
/// # A finish with nothing before it
///
/// A request the daemon could not read, as
/// [`endpoints`](crate::daemon::endpoints) states; or a watch the
/// client cancelled, or whose agent was deleted, before the echo.
///
/// # Error
///
/// An [`Error`](Self::Error) is the absence of a delivery: no agent of
/// the client's is the one named, a structured decision's questions
/// are not valid — the error's `kind` is `questions`, and it names the
/// question — the agent refused the request's content, or no run
/// could start on it, in the daemon's or the agent's own words.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"found":…}`, `{"error":…}` — and
/// a bare variant the string alone — `"not_found"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The agent's program has the request: the run it started took
    /// it, or — a chat message — the program accepted it onto its own
    /// queue, taken at its next seam. Never sent to a request that
    /// watches. Tag `0`.
    Delivered,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// The request was not delivered and will not be. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
    /// The agent's image does not chat: its document states `chat` as
    /// `false`, or leaves it out. Tag `3`.
    ///
    /// No chat message is taken, whatever it carries.
    NoChat,
    /// The request carries what the agent's image does not take: by
    /// [`Lacking`], the modalities its prompt carries and the image's
    /// `prompt_modalities` state as `false`, and — a structured
    /// decision's — those its choices carry and `choice_modalities`
    /// state as `false`. At least one of the two lists is not empty; a
    /// chat message's `choice` always is. Tag `4`.
    ///
    /// The whole request is refused, the text in it included.
    UnsupportedModalities(Lacking),
    /// The agent's image takes no structured decision: its document
    /// states `structured_decisions` as `false`, or leaves it out.
    /// Tag `5`.
    NoStructuredDecisions,
    /// One item of the agent's log, as the log holds it: sent to a
    /// request that watches, from its echo on. Tag `6`.
    Item(ItemWrapper),
    /// The agent's program accepted the request and its run kept
    /// nothing of it: sent to a request that watches, alone. Tag `7`.
    Dropped,
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// Tag for [`Frame::NoChat`].
const NO_CHAT: u8 = 3;

/// Tag for [`Frame::UnsupportedModalities`].
const UNSUPPORTED_MODALITIES: u8 = 4;

/// Tag for [`Frame::NoStructuredDecisions`].
const NO_STRUCTURED_DECISIONS: u8 = 5;

/// Tag for [`Frame::Item`].
const ITEM: u8 = 6;

/// Tag for [`Frame::Dropped`].
const DROPPED: u8 = 7;

/// A tag, and — for the error, the modalities and the item — that
/// variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
            Frame::NoChat => {
                out.extend_from_slice(&[NO_CHAT]);
                Ok(())
            }
            Frame::UnsupportedModalities(lacking) => {
                out.extend_from_slice(&[UNSUPPORTED_MODALITIES]);
                serde_json::to_writer(out, lacking)
            }
            Frame::NoStructuredDecisions => {
                out.extend_from_slice(&[NO_STRUCTURED_DECISIONS]);
                Ok(())
            }
            Frame::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                serde_json::to_writer(out, item)
            }
            Frame::Dropped => {
                out.extend_from_slice(&[DROPPED]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Five ways to fail, and three of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELIVERED => Ok(Frame::Delivered),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            NO_CHAT => Ok(Frame::NoChat),
            UNSUPPORTED_MODALITIES => serde_json::from_slice(rest).map(Frame::UnsupportedModalities).map_err(FrameError::Modalities),
            NO_STRUCTURED_DECISIONS => Ok(Frame::NoStructuredDecisions),
            ITEM => serde_json::from_slice(rest).map(Frame::Item).map_err(FrameError::Item),
            DROPPED => Ok(Frame::Dropped),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A message response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's eight.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The modalities did not parse.
    Modalities(serde_json::Error),
    /// The item did not parse.
    Item(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents message response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents message response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "agents message error did not parse: {error}"),
            FrameError::Modalities(error) => write!(f, "agents message modalities did not parse: {error}"),
            FrameError::Item(error) => write!(f, "agents message item did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) | FrameError::Modalities(error) | FrameError::Item(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
