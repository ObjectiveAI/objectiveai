use serde::{Deserialize, Serialize};

/// Top-level JSONL events emitted by `codex exec`. The wire form is
/// `{"type": <discriminator>, ...payload}` — each leaf struct carries its
/// own `r#type` field, so the parent enum is `untagged` and serde
/// dispatches by trying each variant in declaration order.
///
/// The [`Self::Unknown`] variant mirrors `UnknownThreadEvent` in
/// `types.py:290-295`: any event whose `type` we don't recognise still
/// parses, preserving the discriminator for forward compatibility.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ThreadEvent {
    Known(KnownThreadEvent),
    Unknown(UnknownThreadEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum KnownThreadEvent {
    ThreadStarted(super::ThreadStartedEvent),
    TurnStarted(super::TurnStartedEvent),
    TurnCompleted(super::TurnCompletedEvent),
    TurnFailed(super::TurnFailedEvent),
    ItemStarted(super::ItemStartedEvent),
    ItemUpdated(super::ItemUpdatedEvent),
    ItemCompleted(super::ItemCompletedEvent),
    Error(super::ThreadErrorEvent),
}

/// Forward-compat fallback for events with a `type` we don't recognise.
/// Mirrors `UnknownThreadEvent` in `types.py:290-295`. Only the
/// discriminator is preserved — extra payload fields are discarded since
/// the consumer can't act on them anyway.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnknownThreadEvent {
    pub r#type: String,
}
