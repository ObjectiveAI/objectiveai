//! A dependency tool template's grants, as grants of the daemon's.

use super::{Grant, accounts, agents, agents_templates, dependency_tools, roles, standalone_tools, standalone_tools_templates, vault, volumes};
use crate::shared::permission::{self, Tags, Within};

/// A dependency tool template's grants, as the daemon judges them:
/// the daemon's grants, each the same actions reaching by the kind's
/// own filter carrying the tags alone, and — apart from them, since
/// no grant of the daemon's own can say it — the actions over agents
/// that reach `self`, which the daemon judges by the dependency's own
/// agents: the one a transient tool was deployed for, every one
/// attached to a service. `never_active` is a daemon grant of its own,
/// its filter `{"active": "never"}`. Nothing is widened: a filter of
/// tags alone reaches what the tags test reaches and nothing else,
/// `self` reaches the dependency's agents and nothing else, and
/// `never_active` the agents that have never run and nothing else.
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
                    permission::agents::Within::NeverActive => Within::Only(never_active()),
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
        permission::Grant::StandaloneTools(permission) => Grant::StandaloneTools(match permission {
            permission::standalone_tools::Permission::Make(makes) => standalone_tools::Permission::Make(makes),
            permission::standalone_tools::Permission::Over { actions, within } => standalone_tools::Permission::Over {
                actions,
                within: within.map(standalone_tools_filter),
            },
            permission::standalone_tools::Permission::Tags { actions, within, tags } => standalone_tools::Permission::Tags {
                actions,
                within: within.map(standalone_tools_filter),
                tags,
            },
        }),
        permission::Grant::StandaloneToolsTemplates(permission) => Grant::StandaloneToolsTemplates(match permission {
            permission::standalone_tools_templates::Permission::Make(makes) => standalone_tools_templates::Permission::Make(makes),
            permission::standalone_tools_templates::Permission::Over { actions, within } => standalone_tools_templates::Permission::Over {
                actions,
                within: within.map(standalone_tools_templates_filter),
            },
            permission::standalone_tools_templates::Permission::Tags { actions, within, tags } => standalone_tools_templates::Permission::Tags {
                actions,
                within: within.map(standalone_tools_templates_filter),
                tags,
            },
        }),
        permission::Grant::DependencyTools(permission) => Grant::DependencyTools(match permission {
            permission::dependency_tools::Permission::Over { actions, within } => dependency_tools::Permission::Over {
                actions,
                within: within.map(dependency_tools_filter),
            },
            permission::dependency_tools::Permission::Tags { actions, within, tags } => dependency_tools::Permission::Tags {
                actions,
                within: within.map(dependency_tools_filter),
                tags,
            },
        }),
        permission::Grant::Providers(permission) => Grant::Providers(permission),
        permission::Grant::Daemons(permission) => Grant::Daemons(permission),
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
        permission::Grant::Vault(permission) => Grant::Vault(match permission {
            permission::vault::Permission::Make(makes) => vault::Permission::Make(makes),
            permission::vault::Permission::Over { actions, within } => vault::Permission::Over {
                actions,
                within: within.map(vault_filter),
            },
            permission::vault::Permission::Tags { actions, within, tags } => vault::Permission::Tags {
                actions,
                within: within.map(vault_filter),
                tags,
            },
        }),
        permission::Grant::Postgres(permission) => Grant::Postgres(permission),
        permission::Grant::Events(permission) => Grant::Events(permission),
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

/// The agents that have never been active, as the daemon's own agents
/// filter states them: what `never_active` reaches.
fn never_active() -> crate::daemon::endpoints::agents::list::client::request::Filter {
    use crate::daemon::endpoints::agents::list::client::request::{Active, Filter, History};
    Filter {
        active: Some(Active::History(History::Never)),
        ..Default::default()
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

fn standalone_tools_filter(tags: Tags) -> crate::daemon::endpoints::standalone_tools::list::client::request::Filter {
    crate::daemon::endpoints::standalone_tools::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn standalone_tools_templates_filter(tags: Tags) -> crate::daemon::endpoints::standalone_tools::templates::list::client::request::Filter {
    crate::daemon::endpoints::standalone_tools::templates::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}

fn dependency_tools_filter(tags: Tags) -> crate::daemon::endpoints::dependency_tools::list::client::request::Filter {
    crate::daemon::endpoints::dependency_tools::list::client::request::Filter {
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

fn vault_filter(tags: Tags) -> crate::daemon::endpoints::vault::list::client::request::Filter {
    crate::daemon::endpoints::vault::list::client::request::Filter {
        all_tags: tags.all_tags,
        any_tags: tags.any_tags,
        ..Default::default()
    }
}
