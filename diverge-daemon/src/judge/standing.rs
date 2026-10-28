//! The account as of one request.

use std::collections::HashSet;

use diverge_sdk::daemon::creator::{Client, Creator};
use diverge_sdk::daemon::grant::{self, Grant};
use diverge_sdk::shared::Identity;
use sqlx::PgConnection;

use super::{ToolRef, Who};
use crate::store::{self, AgentId, dependency_tools, tools};

/// Who is asking, for the length of one request: the identity they
/// are served under, every grant they hold, and — a tool's — the
/// actions over agents it holds within `attached` and which agents
/// those are. An account's grants are every grant of every role it
/// holds, read inside the request's transaction so that the judgment
/// and the write see one state; `None` is an account deleted since
/// its client was admitted, which every handler answers as
/// `Forbidden`. A tool's are what its container declared at its
/// registration, fixed for the run, and its attached agents are read
/// as of the request: a standalone tool's, those it is attached to
/// directly and every agent of every template it is attached to; a
/// dependency tool's, those attached to its row — its one agent for a
/// transient tool, every agent attached to a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// The account's identity, for an account; none for a tool, which
    /// is no account.
    pub identity: Option<Identity>,
    /// What the requests make things as: the client, for an account;
    /// the tool itself, for a tool.
    pub creator: Creator,
    /// Every grant, in no order.
    grants: Vec<Grant>,
    /// The actions over agents held within `attached`, in no order: a
    /// tool's, and none for an account.
    attached_actions: Vec<grant::agents::Over>,
    /// The agents `attached` reaches: a tool's attached agents, and
    /// none for an account.
    attached: HashSet<AgentId>,
    /// Whether this is an administrator, which every judgment allows.
    admin: bool,
}

impl Standing {
    /// The standing of `who` now: an account's read fresh, a tool's as
    /// its container declared it, with its attached agents read fresh.
    pub async fn of(conn: &mut PgConnection, who: Who) -> Result<Option<Standing>, store::Error> {
        match who {
            Who::Account(id) => Ok(store::of_account(conn, id).await?.map(|(identity, grants)| Standing {
                creator: Creator::Client(Client { identity: identity.clone() }),
                identity: Some(identity),
                grants,
                attached_actions: Vec::new(),
                attached: HashSet::new(),
                admin: false,
            })),
            Who::Tool(declared) => {
                let attached = match declared.tool {
                    ToolRef::Standalone(tool) => tools::attached_agents(conn, tool).await?.into_iter().collect(),
                    ToolRef::Dependency(tool) => dependency_tools::attachments::attached(conn, tool)
                        .await?
                        .into_iter()
                        .map(|attachment| attachment.agent)
                        .collect(),
                };
                Ok(Some(Standing {
                    identity: None,
                    creator: declared.fixed.creator.clone(),
                    grants: declared.fixed.grants.clone(),
                    attached_actions: declared.fixed.attached_actions.clone(),
                    attached,
                    admin: false,
                }))
            }
            Who::Admin(identity) => Ok(Some(Standing {
                creator: Creator::Client(Client { identity: identity.clone() }),
                identity: Some(identity),
                grants: Vec::new(),
                attached_actions: Vec::new(),
                attached: HashSet::new(),
                admin: true,
            })),
        }
    }

    /// Whether this is an administrator: every judge allows it
    /// everything, with no grant read.
    pub fn admin(&self) -> bool {
        self.admin
    }

    /// The actions over agents held within `attached`.
    pub fn attached_actions(&self) -> &[grant::agents::Over] {
        &self.attached_actions
    }

    /// Whether the agent is one `attached` reaches.
    pub fn is_attached(&self, agent: AgentId) -> bool {
        self.attached.contains(&agent)
    }

    /// The grants over accounts.
    pub fn accounts(&self) -> impl Iterator<Item = &grant::accounts::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Accounts(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over roles.
    pub fn roles(&self) -> impl Iterator<Item = &grant::roles::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Roles(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over the providers.
    pub fn providers(&self) -> impl Iterator<Item = &grant::providers::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Providers(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants through the remote daemons.
    pub fn daemons(&self) -> impl Iterator<Item = &grant::daemons::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Daemons(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over agent templates.
    pub fn agents_templates(&self) -> impl Iterator<Item = &grant::agents_templates::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::AgentsTemplates(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over tool templates.
    pub fn standalone_tools_templates(&self) -> impl Iterator<Item = &grant::standalone_tools_templates::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::StandaloneToolsTemplates(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over dependency templates.
    pub fn dependency_tools(&self) -> impl Iterator<Item = &grant::dependency_tools::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::DependencyTools(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over agents.
    pub fn agents(&self) -> impl Iterator<Item = &grant::agents::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Agents(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over tools.
    pub fn standalone_tools(&self) -> impl Iterator<Item = &grant::standalone_tools::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::StandaloneTools(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over the database.
    pub fn postgres(&self) -> impl Iterator<Item = &grant::postgres::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Postgres(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over volumes.
    pub fn volumes(&self) -> impl Iterator<Item = &grant::volumes::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Volumes(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over the vault.
    pub fn vault(&self) -> impl Iterator<Item = &grant::vault::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Vault(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over the events.
    pub fn events(&self) -> impl Iterator<Item = &grant::events::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Events(permission) => Some(permission),
            _ => None,
        })
    }

    /// Every grant over images the standing holds.
    pub fn images(&self) -> impl Iterator<Item = &grant::images::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::Images(permission) => Some(permission),
            _ => None,
        })
    }
}
