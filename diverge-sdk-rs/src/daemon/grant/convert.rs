//! A dependency tool template's grants, as grants of the daemon's.

use super::{Grant, accounts, agents, agents_templates, providers_daemons, providers_incoming, providers_outgoing, roles, tools, tools_templates, volumes};
use crate::shared::permission::{self, Tags, Within};

/// A dependency tool template's grants, as the daemon judges them:
/// the daemon's grants, each the same actions reaching by the kind's
/// own filter carrying the tags alone, and — apart from them, since
/// no grant of the daemon's own can say it — the actions over agents
/// that reach `self`, which the daemon judges by the dependency's own
/// agents: the one a transient tool was deployed for, every one
/// attached to a service. Nothing is widened: a filter of tags alone
/// reaches what the tags test reaches and nothing else, and `self`
/// reaches the dependency's agents and nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Converted {
    /// The grants a role could hold, judged as a role's are.
    pub grants: Vec<Grant>,
    /// The actions over agents held within `self`, judged against the
    /// dependency's own agents.
    pub self_actions: Vec<agents::Over>,
}

impl From<Vec<permission::Grant>> for Converted {
    fn from(grants: Vec<permission::Grant>) -> Self {
        let mut converted = Converted::default();
        for grant in grants {
            match daemon_grant(grant) {
                Ok(grant) => converted.grants.push(grant),
                Err(actions) => converted.self_actions.extend(actions),
            }
        }
        converted
    }
}

/// The daemon's grant a shared grant is, or — for a grant over agents
/// within `self`, which no daemon grant can say — the actions it
/// holds, to be judged apart.
fn daemon_grant(grant: permission::Grant) -> Result<Grant, Vec<agents::Over>> {
    Ok(match grant {
        permission::Grant::Agents(permission) => Grant::Agents(match permission {
            permission::agents::Permission::Make(makes) => agents::Permission::Make(makes),
            permission::agents::Permission::Over { actions, within } => {
                let within = match within {
                    permission::agents::Within::Any => Within::Any,
                    permission::agents::Within::Self_ => return Err(actions),
                    permission::agents::Within::Only(tags) => Within::Only(agents_filter(tags)),
                };
                agents::Permission::Over { actions, within }
            }
            permission::agents::Permission::Tags { actions, within, tags } => agents::Permission::Tags {
                actions,
                within: within.map(agents_filter),
                tags,
            },
        }),
        permission::Grant::AgentsTemplates(permission) => Grant::AgentsTemplates(match permission {
            permission::agents_templates::Permission::Make(makes) => agents_templates::Permission::Make(makes),
            permission::agents_templates::Permission::Over { actions, within } => agents_templates::Permission::Over {
                actions,
                within: within.map(agents_templates_filter),
            },
            permission::agents_templates::Permission::Tags { actions, within, tags } => agents_templates::Permission::Tags {
                actions,
                within: within.map(agents_templates_filter),
                tags,
            },
        }),
        permission::Grant::Tools(permission) => Grant::Tools(match permission {
            permission::tools::Permission::Make(makes) => tools::Permission::Make(makes),
            permission::tools::Permission::Over { actions, within } => tools::Permission::Over {
                actions,
                within: within.map(tools_filter),
            },
            permission::tools::Permission::Tags { actions, within, tags } => tools::Permission::Tags {
                actions,
                within: within.map(tools_filter),
                tags,
            },
        }),
        permission::Grant::ToolsTemplates(permission) => Grant::ToolsTemplates(match permission {
            permission::tools_templates::Permission::Make(makes) => tools_templates::Permission::Make(makes),
            permission::tools_templates::Permission::Over { actions, within } => tools_templates::Permission::Over {
                actions,
                within: within.map(tools_templates_filter),
            },
            permission::tools_templates::Permission::Tags { actions, within, tags } => tools_templates::Permission::Tags {
                actions,
                within: within.map(tools_templates_filter),
                tags,
            },
        }),
        permission::Grant::ProvidersOutgoing(permission) => Grant::ProvidersOutgoing(match permission {
            permission::providers_outgoing::Permission::Make(makes) => providers_outgoing::Permission::Make(makes),
            permission::providers_outgoing::Permission::Over { actions, within } => providers_outgoing::Permission::Over {
                actions,
                within: within.map(providers_outgoing_filter),
            },
            permission::providers_outgoing::Permission::Tags { actions, within, tags } => providers_outgoing::Permission::Tags {
                actions,
                within: within.map(providers_outgoing_filter),
                tags,
            },
        }),
        permission::Grant::ProvidersIncoming(permission) => Grant::ProvidersIncoming(match permission {
            permission::providers_incoming::Permission::Make(makes) => providers_incoming::Permission::Make(makes),
            permission::providers_incoming::Permission::Over { actions, within } => providers_incoming::Permission::Over {
                actions,
                within: within.map(providers_incoming_filter),
            },
            permission::providers_incoming::Permission::Tags { actions, within, tags } => providers_incoming::Permission::Tags {
                actions,
                within: within.map(providers_incoming_filter),
                tags,
            },
        }),
        permission::Grant::ProvidersDaemons(permission) => Grant::ProvidersDaemons(match permission {
            permission::providers_daemons::Permission::Make(makes) => providers_daemons::Permission::Make(makes),
            permission::providers_daemons::Permission::Over { actions, within } => providers_daemons::Permission::Over {
                actions,
                within: within.map(providers_daemons_filter),
            },
            permission::providers_daemons::Permission::Tags { actions, within, tags } => providers_daemons::Permission::Tags {
                actions,
                within: within.map(providers_daemons_filter),
                tags,
            },
        }),
        permission::Grant::Accounts(permission) => Grant::Accounts(match permission {
            permission::accounts::Permission::Make(makes) => accounts::Permission::Make(makes),
            permission::accounts::Permission::Over { actions, within } => accounts::Permission::Over {
                actions,
                within: within.map(accounts_filter),
            },
            permission::accounts::Permission::Tags { actions, within, tags } => accounts::Permission::Tags {
                actions,
                within: within.map(accounts_filter),
                tags,
            },
        }),
        permission::Grant::Roles(permission) => Grant::Roles(match permission {
            permission::roles::Permission::Make(makes) => roles::Permission::Make(makes),
            permission::roles::Permission::Over { actions, within } => roles::Permission::Over {
                actions,
                within: within.map(roles_filter),
            },
            permission::roles::Permission::Tags { actions, within, tags } => roles::Permission::Tags {
                actions,
                within: within.map(roles_filter),
                tags,
            },
        }),
        permission::Grant::Volumes(permission) => Grant::Volumes(match permission {
            permission::volumes::Permission::Make(makes) => volumes::Permission::Make(makes),
            permission::volumes::Permission::Over { actions, within } => volumes::Permission::Over {
                actions,
                within: within.map(volumes_filter),
            },
            permission::volumes::Permission::Tags { actions, within, tags } => volumes::Permission::Tags {
                actions,
                within: within.map(volumes_filter),
                tags,
            },
        }),
        permission::Grant::Postgres(permission) => Grant::Postgres(permission),
    })
}

impl<T> Within<T> {
    /// `"any"` as it is, and what is named carried through `f`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Within<U> {
        match self {
            Within::Any => Within::Any,
            Within::Only(only) => Within::Only(f(only)),
        }
    }
}

fn agents_filter(tags: Tags) -> crate::daemon::endpoints::agents::list::client::request::Filter {
    crate::daemon::endpoints::agents::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn agents_templates_filter(tags: Tags) -> crate::daemon::endpoints::agents::templates::list::client::request::Filter {
    crate::daemon::endpoints::agents::templates::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn tools_filter(tags: Tags) -> crate::daemon::endpoints::tools::list::client::request::Filter {
    crate::daemon::endpoints::tools::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn tools_templates_filter(tags: Tags) -> crate::daemon::endpoints::tools::templates::list::client::request::Filter {
    crate::daemon::endpoints::tools::templates::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn providers_outgoing_filter(tags: Tags) -> crate::daemon::endpoints::providers::outgoing::list::client::request::Filter {
    crate::daemon::endpoints::providers::outgoing::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn providers_incoming_filter(tags: Tags) -> crate::daemon::endpoints::providers::incoming::list::client::request::Filter {
    crate::daemon::endpoints::providers::incoming::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn providers_daemons_filter(tags: Tags) -> crate::daemon::endpoints::providers::daemons::list::client::request::Filter {
    crate::daemon::endpoints::providers::daemons::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn accounts_filter(tags: Tags) -> crate::daemon::endpoints::accounts::list::client::request::Filter {
    crate::daemon::endpoints::accounts::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn roles_filter(tags: Tags) -> crate::daemon::endpoints::roles::list::client::request::Filter {
    crate::daemon::endpoints::roles::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn volumes_filter(tags: Tags) -> crate::daemon::endpoints::volumes::list::client::request::Filter {
    crate::daemon::endpoints::volumes::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}
