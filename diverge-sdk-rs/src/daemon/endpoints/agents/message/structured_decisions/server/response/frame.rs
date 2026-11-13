//! What a server's response frame carries for a structured decision.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::info::Lacking;
use crate::shared::error::Error;

/// A structured decision's answer: the agent's program has it, or a
/// failure. Not the agent's answers to the questions, which are said
/// in its run and read in its log.
///
/// One of these per scope, before the finish that ends it — and the
/// reply comes when the fate is known, which may be long after the
/// ask: a decision waits for everything the agent was handed before
/// it to finish, and for the agent to be idle, before the run it
/// starts is begun, and nothing times it out. A payload leads with one
/// byte saying which — `0` for [`Delivered`](Self::Delivered), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error), `3`
/// for [`NoStructuredDecisions`](Self::NoStructuredDecisions), `4` for
/// [`UnsupportedModalities`](Self::UnsupportedModalities) — and only
/// the error and the modalities carry anything after it.
///
/// # What the agent's image says
///
/// The daemon holds the decision to the `structured_decisions` of the
/// agent's image's `/DIVERGE.md`, as it has it on record, before
/// anything is sent on:
/// [`NoStructuredDecisions`](Self::NoStructuredDecisions) where the
/// image states `false`, and
/// [`UnsupportedModalities`](Self::UnsupportedModalities) where the
/// decision carries what the image does not take. Either way the
/// decision reached no provider and no container, nothing was started
/// for it, and the agent's log has no line of it.
///
/// # A finish with nothing before it
///
/// A request the daemon could not read, as
/// [`endpoints`](crate::daemon::endpoints) states. Nothing else ends
/// the scope without an answer: nothing takes a decision back.
///
/// # Error
///
/// An [`Error`](Self::Error) is the absence of a delivery: no agent of
/// the client's is the one named, the questions are not valid — the
/// error's `kind` is `questions`, and it names the question — the
/// agent refused the decision, or no run could start on it, in the
/// daemon's or the agent's own words.
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
    /// The agent's program has the decision: the run it starts has
    /// begun. Tag `0`.
    Delivered,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// The decision was not delivered and will not be. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
    /// The agent's image makes no structured decisions: its document
    /// states `structured_decisions` as `false`, or leaves it out.
    /// Tag `3`.
    ///
    /// No decision is taken, whatever it carries.
    NoStructuredDecisions,
    /// The decision carries what the agent's image does not take: see
    /// [`Lacking`], the modalities its prompt carries and the image's
    /// `prompt_modalities` state as `false`, and those its choices
    /// carry and `choice_modalities` state as `false`. At least one of
    /// the two lists is not empty. Tag `4`.
    ///
    /// The whole decision is refused.
    UnsupportedModalities(Lacking),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// Tag for [`Frame::NoStructuredDecisions`].
const NO_STRUCTURED_DECISIONS: u8 = 3;

/// Tag for [`Frame::UnsupportedModalities`].
const UNSUPPORTED_MODALITIES: u8 = 4;

/// A tag, and — for the error and the modalities — that variant's own
/// JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The three bare answers cannot fail.
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
            Frame::NoStructuredDecisions => {
                out.extend_from_slice(&[NO_STRUCTURED_DECISIONS]);
                Ok(())
            }
            Frame::UnsupportedModalities(modalities) => {
                out.extend_from_slice(&[UNSUPPORTED_MODALITIES]);
                serde_json::to_writer(out, modalities)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and two of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELIVERED => Ok(Frame::Delivered),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            NO_STRUCTURED_DECISIONS => Ok(Frame::NoStructuredDecisions),
            UNSUPPORTED_MODALITIES => serde_json::from_slice(rest).map(Frame::UnsupportedModalities).map_err(FrameError::Modalities),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A structured decisions response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
    /// The modalities did not parse.
    Modalities(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents message structured decisions response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents message structured decisions response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "agents message structured decisions error did not parse: {error}"),
            FrameError::Modalities(error) => write!(f, "agents message structured decisions modalities did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) | FrameError::Modalities(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
