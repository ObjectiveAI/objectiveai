//! Judging requests over routes.
//!
//! Two shapes, since a route carries no tags: a making action — the
//! set — is held or it is not; an action over a route is allowed by a
//! grant that holds it and whose `within` reaches the route — `any`,
//! or the routes list filter as [`filter::routes::test`] reads it,
//! with the name of the tool the route names, which the caller knows.

use diverge_sdk::daemon::endpoints::tools::routes::list::client::request::Filter;
use diverge_sdk::daemon::grant::Within;
use diverge_sdk::daemon::grant::routes::{Make, Over, Permission};

use super::Standing;
use super::filter;
use crate::store::routes::Route;

/// Whether the standing may put a route down.
pub fn create(standing: &Standing) -> bool {
    standing
        .routes()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Set)))
}

/// Whether the standing holds `action` over any route at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .routes()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `route`, given the name
/// of its tool as it is called now.
pub fn over(standing: &Standing, action: Over, route: &Route, tool_name: Option<&str>) -> bool {
    standing.routes().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, route, tool_name),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the route.
fn reaches(within: &Within<Filter>, route: &Route, tool_name: Option<&str>) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::routes::test(filter, route, tool_name),
    }
}
