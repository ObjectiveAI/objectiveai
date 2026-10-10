//! One grant, by the kind it is over.

use serde::{Deserialize, Serialize};

use super::{agents, agents_templates, tools, tools_templates, providers_outgoing, providers_incoming, accounts, roles, volumes, postgres};

/// One grant: a permission over one kind of thing the daemon holds.
/// Externally tagged by the kind's name — one object with one member,
/// `{"agents":…}`, `{"providers_incoming":…}` — whose value is that
/// kind's [`Permission`](agents::Permission), in one of the shapes
/// [`grant`](crate::daemon::grant) states. Snake case on the wire:
/// `"agents"`, `"agents_templates"`, `"tools"`, `"tools_templates"`,
/// `"providers_outgoing"`, `"providers_incoming"`, `"accounts"`,
/// `"roles"`, `"volumes"`, `"postgres"` — the kinds of
/// [`shared::permission::Grant`](crate::shared::permission::Grant),
/// which this is built of.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grant {
    /// Over agents: see [`agents::Permission`].
    Agents(agents::Permission),
    /// Over agent templates: see [`agents_templates::Permission`].
    AgentsTemplates(agents_templates::Permission),
    /// Over tools: see [`tools::Permission`].
    Tools(tools::Permission),
    /// Over tool templates: see [`tools_templates::Permission`].
    ToolsTemplates(tools_templates::Permission),
    /// Over outgoing providers: see [`providers_outgoing::Permission`].
    ProvidersOutgoing(providers_outgoing::Permission),
    /// Over incoming credentials: see [`providers_incoming::Permission`].
    ProvidersIncoming(providers_incoming::Permission),
    /// Over accounts: see [`accounts::Permission`].
    Accounts(accounts::Permission),
    /// Over roles: see [`roles::Permission`].
    Roles(roles::Permission),
    /// Over volumes: see [`volumes::Permission`].
    Volumes(volumes::Permission),
    /// Over the database: see [`postgres::Permission`].
    Postgres(postgres::Permission),
}
