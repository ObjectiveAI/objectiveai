//! Judging requests over volumes.
//!
//! Two shapes, since a volume carries no tags: a making action — the
//! create — is held or it is not, judged by nothing else; an action
//! over a volume is allowed by a grant that holds it and whose
//! `within` reaches the volume — `any`, or the volumes list filter as
//! [`filter::volumes::test`] reads it, over the volume as a list
//! reports it, the agents and tools that mount it included.

use diverge_sdk::daemon::endpoints::volumes::list::client::request::Filter;
use diverge_sdk::daemon::grant::Within;
use diverge_sdk::daemon::grant::volumes::{Make, Over, Permission};

use super::Standing;
use super::filter;
use crate::volumes::Listed;

/// Whether the standing may create a volume.
pub fn create(standing: &Standing) -> bool {
    standing
        .volumes()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Create)))
}

/// Whether the standing holds `action` over any volume at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .volumes()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the volume.
pub fn over(standing: &Standing, action: Over, listed: &Listed) -> bool {
    standing.volumes().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, listed),
        _ => false,
    })
}

/// The grants' reach over providers, for a list: `None` for every
/// provider — some grant of `action` is `any`, or names none — else
/// the providers the grants name, which is where a listing looks.
pub fn providers(standing: &Standing, action: Over) -> Option<Vec<diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity>> {
    let mut named = Vec::new();
    for permission in standing.volumes() {
        let Permission::Over { actions, within } = permission else {
            continue;
        };
        if !actions.contains(&action) {
            continue;
        }
        match within {
            Within::Any => return None,
            Within::Only(filter) if filter.providers.is_empty() => return None,
            Within::Only(filter) => named.extend(filter.providers.iter().cloned()),
        }
    }
    Some(named)
}

/// Whether a grant's `within` reaches the volume.
fn reaches(within: &Within<Filter>, listed: &Listed) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::volumes::test(filter, listed),
    }
}
