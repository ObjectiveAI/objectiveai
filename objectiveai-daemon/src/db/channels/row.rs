//! Neutral row + enum types for the `channels` DB layer. Deliberately
//! free of the SDK `channels` command types (which don't exist until
//! the command tier is built) — the daemon handlers map these to the
//! wire response types.

use objectiveai_sdk::identity::Identity;

/// Which side of a channel a message came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Publisher → owner.
    Request,
    /// Owner → publisher.
    Reply,
}

impl Direction {
    /// The stored TEXT token.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Request => "request",
            Direction::Reply => "reply",
        }
    }

    /// Parse the stored TEXT token.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "request" => Some(Direction::Request),
            "reply" => Some(Direction::Reply),
            _ => None,
        }
    }
}

/// Which role a presented secret authenticates as on a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Holder of `pub_secret` — writes requests, reads replies.
    Publisher,
    /// Holder of `owner_secret` — writes replies, reads requests.
    Owner,
}

/// A channel's lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Open,
    Closed,
}

impl ChannelState {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(ChannelState::Open),
            "closed" => Some(ChannelState::Closed),
            _ => None,
        }
    }
}

/// A channel's authentication + lifecycle snapshot — everything the
/// command handlers need to authorize an operation.
pub struct ChannelAuth {
    pub pub_secret: String,
    pub owner_secret: String,
    pub state: ChannelState,
}

impl ChannelAuth {
    /// Which role (if any) the presented secret authenticates as.
    pub fn role_of(&self, secret: &str) -> Option<Role> {
        if secret == self.pub_secret {
            Some(Role::Publisher)
        } else if secret == self.owner_secret {
            Some(Role::Owner)
        } else {
            None
        }
    }
}

/// One channel-log entry's ENVELOPE — what `logs list` returns (no
/// content).
pub struct MessageEnvelope {
    pub id: i64,
    pub direction: Direction,
    pub identity: Identity,
    /// Unix-seconds.
    pub delivered_at: i64,
}

/// One channel-log entry's CONTENT — what `logs open` reveals.
pub struct MessageContent {
    pub id: i64,
    pub direction: Direction,
    pub identity: Identity,
    /// Unix-seconds.
    pub delivered_at: i64,
    pub content: serde_json::Value,
}
