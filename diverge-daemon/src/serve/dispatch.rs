//! Every request, to its handler.

use diverge_sdk::daemon::endpoints::ClientRequest;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::{accounts, agents, postgres, providers, resources, roles, tools, volumes};
use crate::daemon::Daemon;
use crate::judge::Who;

/// Read `payload` as the request that opened `scope` and hand it to
/// its handler, which answers and finishes the scope.
///
/// One arm per request of the wire, so that a request added to the
/// SDK is a request this cannot compile without. Every one of the
/// ninety has a handler. A payload that is no request at all is
/// finished with nothing before it, which is what the wire means by a
/// request that was not served.
pub async fn dispatch(scope: ScopeHandle, payload: &[u8], who: Who, daemon: &Arc<Daemon>) {
    match ClientRequest::decode(payload).unwrap_or_else(|never| match never {}) {
        ClientRequest::AgentsCreate(frame) => agents::create::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsGet(frame) => agents::get::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsDelete(frame) => agents::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsMessage(frame) => agents::message::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsLogs(frame) => agents::logs::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsList(frame) => agents::list::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsEdit(frame) => agents::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTag(frame) => agents::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsUntag(frame) => agents::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesCreate(frame) => agents::templates::create::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesGet(frame) => agents::templates::get::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesList(frame) => agents::templates::list::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesDelete(frame) => agents::templates::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesTag(frame) => agents::templates::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTemplatesUntag(frame) => agents::templates::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsCreate(frame) => tools::create::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsGet(frame) => tools::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsEdit(frame) => tools::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsConnect(frame) => tools::connect::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsListFor(frame) => tools::list_for::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsAttach(frame) => tools::attach::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsDetach(frame) => tools::detach::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsDelete(frame) => tools::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsList(frame) => tools::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTag(frame) => tools::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsUntag(frame) => tools::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsRoutesSet(frame) => tools::routes::set::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsRoutesDelete(frame) => tools::routes::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsRoutesList(frame) => tools::routes::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesCreate(frame) => tools::templates::create::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesGet(frame) => tools::templates::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesList(frame) => tools::templates::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesDelete(frame) => tools::templates::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesTag(frame) => tools::templates::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTemplatesUntag(frame) => tools::templates::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesUpload(frame) => resources::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesList(frame) => resources::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesDelete(frame) => resources::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingAdd(frame) => providers::outgoing::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingGet(frame) => providers::outgoing::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingList(frame) => providers::outgoing::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingDelete(frame) => providers::outgoing::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingEdit(frame) => providers::outgoing::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingAdd(frame) => providers::incoming::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingGet(frame) => providers::incoming::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingList(frame) => providers::incoming::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingDelete(frame) => providers::incoming::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingEdit(frame) => providers::incoming::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsCreate(frame) => accounts::create::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsGet(frame) => accounts::get::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsList(frame) => accounts::list::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsDelete(frame) => accounts::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsEdit(frame) => accounts::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsTag(frame) => accounts::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::AccountsUntag(frame) => accounts::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesCreate(frame) => roles::create::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesGet(frame) => roles::get::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesList(frame) => roles::list::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesDelete(frame) => roles::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesEdit(frame) => roles::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesTag(frame) => roles::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::RolesUntag(frame) => roles::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsDownload(frame) => agents::download::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsUpload(frame) => agents::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsTransfer(frame) => agents::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsDownload(frame) => tools::download::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsUpload(frame) => tools::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsTransfer(frame) => tools::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesDownload(frame) => resources::download::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesTransfer(frame) => resources::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesCreate(frame) => volumes::create::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesGet(frame) => volumes::get::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesList(frame) => volumes::list::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesDelete(frame) => volumes::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesEdit(frame) => volumes::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesStat(frame) => volumes::stat::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesDownload(frame) => volumes::download::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesUpload(frame) => volumes::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesTransfer(frame) => volumes::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsFiletree(frame) => agents::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsFiletree(frame) => tools::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesFiletree(frame) => resources::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesFiletree(frame) => volumes::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesGet(frame) => resources::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesTag(frame) => resources::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesUntag(frame) => resources::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::PostgresGet(frame) => postgres::get::handle(scope, frame, who, daemon).await,
        ClientRequest::PostgresConnections(frame) => postgres::connections::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsAdmit(frame) => tools::admit::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsUnadmit(frame) => tools::unadmit::handle(scope, frame, who, daemon).await,
        ClientRequest::Invalid(_) => scope.send_response_finish().await,
    }
}
