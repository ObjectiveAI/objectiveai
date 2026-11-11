//! What the courier asks of the daemon, declared at its registration.

use diverge_sdk::shared::permission::Grant;
use diverge_sdk::shared::permission::agents::{Over, Permission, Within};

/// The courier's permissions: `get`, `list` and `message` over the
/// agents attached to it — those whose registration declared it — and
/// over the agents that have never run, so they can be listed and
/// written to before their first start, which the notification
/// causes. Nothing else.
pub fn permissions() -> Vec<Grant> {
    let actions = || vec![Over::Get, Over::List, Over::Message];
    vec![
        Grant::Agents(Permission::Over {
            actions: actions(),
            within: Within::Attached,
        }),
        Grant::Agents(Permission::Over {
            actions: actions(),
            within: Within::NeverActive,
        }),
    ]
}
