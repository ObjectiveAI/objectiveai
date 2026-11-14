//! What the chat asks of the daemon, declared at its registration.

use diverge_sdk::shared::permission::{Grant, Within, agents, agents_templates, daemons, providers};

/// The chat's permissions:
///
/// 1. `get`, `list` and `message` over the agents attached to it: the
///    page's tabs are the attached agents, and a person's message is
///    announced to its agent.
/// 2. `get` over every agent template: an agent's image is its
///    template's, which the page names and describes.
/// 3. `read` over every provider and every remote daemon: an image's
///    `/DIVERGE.md`, its name and its document, which the page shows,
///    is read through whichever of them holds the image.
pub fn permissions() -> Vec<Grant> {
    vec![
        Grant::Agents(agents::Permission::Over { actions: vec![agents::Over::Get, agents::Over::List, agents::Over::Message], within: agents::Within::Attached }),
        Grant::AgentsTemplates(agents_templates::Permission::Over { actions: vec![agents_templates::Over::Get], within: Within::Any }),
        Grant::Providers(providers::Permission::Over { actions: vec![providers::Over::Read], within: Within::Any }),
        Grant::Daemons(daemons::Permission::Over { actions: vec![daemons::Over::Read], within: Within::Any }),
    ]
}
