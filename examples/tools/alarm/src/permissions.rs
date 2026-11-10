//! What the alarm tool asks of the daemon, declared at its
//! registration.

use diverge_sdk::shared::permission::Grant;
use diverge_sdk::shared::permission::agents::{Over, Permission, Within};

/// The alarm tool's permissions: `message` over the agents attached to
/// it, which is how an alarm wakes its agent. Nothing else.
pub fn permissions() -> Vec<Grant> {
    vec![Grant::Agents(Permission::Over {
        actions: vec![Over::Message],
        within: Within::Attached,
    })]
}
