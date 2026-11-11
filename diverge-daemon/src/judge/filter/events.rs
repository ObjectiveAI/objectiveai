//! The events filter, as a test.

use diverge_sdk::daemon::endpoints::events::list::client::request::Filter;

use crate::store::events::Event;

/// Whether `event` passes `filter`: its sender or its receiver is any
/// one of `parties`; its sender is any one of `senders`; its receiver
/// is any one of `receivers` — every member given holding at once,
/// and a member that is empty passing everything.
pub fn test(filter: &Filter, event: &Event) -> bool {
    (filter.parties.is_empty() || filter.parties.contains(&event.sender) || filter.parties.contains(&event.receiver))
        && (filter.senders.is_empty() || filter.senders.contains(&event.sender))
        && (filter.receivers.is_empty() || filter.receivers.contains(&event.receiver))
}
