//! One grant, by the kind it is over.

use serde::{Deserialize, Serialize};

use super::{accounts, agents, agents_templates, postgres, providers_incoming, providers_outgoing, roles, tools, tools_templates, volumes};

/// One grant that names nothing: a permission over one kind of thing
/// the daemon holds, reaching by tags alone. Externally tagged by the
/// kind's name — one object with one member, `{"agents":…}`,
/// `{"volumes":…}` — whose value is that kind's
/// [`Permission`](agents::Permission), in one of the shapes
/// [`permission`](crate::shared::permission) states. Snake case on the
/// wire: `"agents"`, `"agents_templates"`, `"tools"`,
/// `"tools_templates"`, `"providers_outgoing"`, `"providers_incoming"`,
/// `"accounts"`, `"roles"`, `"volumes"`, `"postgres"`. The daemon's
/// [`Grant`](crate::daemon::grant::Grant) has these kinds and one more,
/// routes, which names positions and cannot travel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
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
