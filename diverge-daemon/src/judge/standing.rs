//! The account as of one request.

use std::collections::HashSet;

use diverge_sdk::daemon::grant::{self, Grant};
use sqlx::PgConnection;

use super::Who;
use crate::store::{self, AgentId, dependency_tools};

/// Who is asking, for the length of one request: the identity they
/// are served under, every grant they hold, and — a dependency tool's
/// — the actions over agents it holds within `self` and which agents
/// those are. An account's grants are every grant of every role it
/// holds, read inside the request's transaction so that the judgment
/// and the write see one state; `None` is an account deleted since
/// its client was admitted, which every handler answers as
/// `Forbidden`. A dependency tool's are its template's, fixed when it
/// was deployed, and its selves are the agents attached to its row as
/// of the request: its one agent for a transient tool, every agent
/// attached to a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    /// The account's name when it has one, else its credential's
    /// identity: what a `Creator::Client` carries.
    pub identity: String,
    /// Every grant, in no order.
    grants: Vec<Grant>,
    /// The actions over agents held within `self`, in no order: a
    /// dependency tool's, and none for an account.
    self_actions: Vec<grant::agents::Over>,
    /// The agents `self` reaches: a dependency tool's own, and none for
    /// an account.
    selves: HashSet<AgentId>,
}

impl Standing {
    /// The standing of `who` now: an account's read fresh, a
    /// dependency's as it was fixed, with its selves read fresh.
    pub async fn of(conn: &mut PgConnection, who: Who) -> Result<Option<Standing>, store::Error> {
        match who {
            Who::Account(id) => Ok(store::of_account(conn, id).await?.map(|(identity, grants)| Standing {
                identity,
                grants,
                self_actions: Vec::new(),
                selves: HashSet::new(),
            })),
            Who::Dependency(dependent) => {
                let selves = dependency_tools::attachments::attached(conn, dependent.tool)
                    .await?
                    .into_iter()
                    .map(|attachment| attachment.agent)
                    .collect();
                Ok(Some(Standing {
                    identity: dependent.fixed.identity.clone(),
                    grants: dependent.fixed.grants.clone(),
                    self_actions: dependent.fixed.self_actions.clone(),
                    selves,
                }))
            }
        }
    }

    /// The actions over agents held within `self`.
    pub fn self_actions(&self) -> &[grant::agents::Over] {
        &self.self_actions
    }

    /// Whether the agent is one `self` reaches.
    pub fn is_self(&self, agent: AgentId) -> bool {
        self.selves.contains(&agent)
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

    /// The grants over outgoing providers.
    pub fn providers_outgoing(&self) -> impl Iterator<Item = &grant::providers_outgoing::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersOutgoing(permission) => Some(permission),
            _ => None,
        })
    }

    /// The grants over daemon records.
    pub fn providers_daemons(&self) -> impl Iterator<Item = &grant::providers_daemons::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersDaemons(permission) => Some(permission),
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

    /// The grants over incoming credentials.
    pub fn providers_incoming(&self) -> impl Iterator<Item = &grant::providers_incoming::Permission> {
        self.grants.iter().filter_map(|grant| match grant {
            Grant::ProvidersIncoming(permission) => Some(permission),
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
}
