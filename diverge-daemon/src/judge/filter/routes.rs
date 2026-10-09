//! The routes filter, as a test.

use diverge_sdk::daemon::endpoints::tools::routes::list::client::request::Filter;

use crate::store::routes::Route;

/// Whether `route` passes `filter`, given the name of its tool as it
/// is called now. `templates` is the position's template: the
/// dependency's own.
pub fn test(filter: &Filter, route: &Route, tool_name: Option<&str>) -> bool {
    (filter.agents.is_empty() || filter.agents.contains(&route.agent))
        && (filter.templates.is_empty() || filter.templates.contains(&route.template))
        && (filter.tools.is_empty() || tool_name.is_some_and(|name| filter.tools.iter().any(|wanted| wanted == name)))
        && (filter.creators.is_empty() || filter.creators.contains(&route.creator))
        && filter.created_from.is_none_or(|from| route.created >= from)
        && filter.created_to.is_none_or(|to| route.created <= to)
}
