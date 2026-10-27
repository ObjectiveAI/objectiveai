//! One grant, by the kind it is over.

use serde::{Deserialize, Serialize};

use super::{accounts, agents, agents_templates, dependency_tools, daemons, events, images, postgres, providers, roles, standalone_tools, standalone_tools_templates, vault, volumes};

/// One grant that names nothing: a permission over one kind of thing
/// the daemon holds, reaching by tags alone. Externally tagged by the
/// kind's name — one object with one member, `{"agents":…}`,
/// `{"volumes":…}` — whose value is that kind's
/// [`Permission`](agents::Permission), in one of the shapes
/// [`permission`](crate::shared::permission) states. Snake case on the
/// wire: `"agents"`, `"agents_templates"`, `"standalone_tools"`,
/// `"standalone_tools_templates"`, `"dependency_tools"`,
/// `"providers"`, `"daemons"`, `"accounts"`,
/// `"roles"`, `"volumes"`, `"vault"`, `"postgres"`, `"events"`, `"images"` — the
/// kinds of the daemon's [`Grant`](crate::daemon::grant::Grant), which
/// is made of these.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Grant {
    /// Over agents: see [`agents::Permission`].
    Agents(agents::Permission),
    /// Over agent templates: see [`agents_templates::Permission`].
    AgentsTemplates(agents_templates::Permission),
    /// Over standalone tools: see [`standalone_tools::Permission`].
    StandaloneTools(standalone_tools::Permission),
    /// Over standalone tool templates: see
    /// [`standalone_tools_templates::Permission`].
    StandaloneToolsTemplates(standalone_tools_templates::Permission),
    /// Over dependency tools: see [`dependency_tools::Permission`].
    DependencyTools(dependency_tools::Permission),
    /// Over the daemon's providers: see [`providers::Permission`].
    Providers(providers::Permission),
    /// Over the remote daemons it reaches: see [`daemons::Permission`].
    Daemons(daemons::Permission),
    /// Over accounts: see [`accounts::Permission`].
    Accounts(accounts::Permission),
    /// Over roles: see [`roles::Permission`].
    Roles(roles::Permission),
    /// Over volumes: see [`volumes::Permission`].
    Volumes(volumes::Permission),
    /// Over the vault's keys: see [`vault::Permission`].
    Vault(vault::Permission),
    /// Over the database: see [`postgres::Permission`].
    Postgres(postgres::Permission),
    /// Over the events: see [`events::Permission`].
    Events(events::Permission),
    /// Over images: see [`images::Permission`].
    Images(images::Permission),
}
