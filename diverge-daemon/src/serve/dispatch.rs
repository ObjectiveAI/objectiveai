//! Every request, to its handler — or to the one answer there is.

use diverge_sdk::daemon::endpoints::{self, ClientRequest};
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::{accounts, agents, providers, resources, roles, tools};
use crate::daemon::Daemon;
use crate::judge::Who;

/// What every request nothing serves yet is answered with.
const NOT_SERVED: &str = "not served: the daemon serves accounts, roles, providers, templates, resources, agents, tools and routes, and nothing else yet";

/// Read `payload` as the request that opened `scope` and hand it to
/// its handler, which answers and finishes the scope.
///
/// One arm per request of the wire, so that a request added to the
/// SDK is a request this cannot compile without. The seventy over
/// accounts, roles, providers, templates, resources, agents, tools,
/// routes and a provider's tool containers have handlers; every other is answered with its
/// endpoint's own `Error`, carrying one sentence saying so, encoded as
/// the endpoint encodes it, then the finish. A payload that is no
/// request at all is finished with nothing before it, which is what
/// the wire means by a request that was not served.
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
        ClientRequest::AgentsDownload(_) => refused(scope, encoded(&endpoints::agents::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsUpload(_) => refused(scope, encoded(&endpoints::agents::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsTransfer(_) => refused(scope, encoded(&endpoints::agents::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsDownload(_) => refused(scope, encoded(&endpoints::tools::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsUpload(_) => refused(scope, encoded(&endpoints::tools::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsTransfer(_) => refused(scope, encoded(&endpoints::tools::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesDownload(frame) => resources::download::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesTransfer(frame) => resources::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesCreate(_) => refused(scope, encoded(&endpoints::volumes::create::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesGet(_) => refused(scope, encoded(&endpoints::volumes::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesList(_) => refused(scope, encoded(&endpoints::volumes::list::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesDelete(_) => refused(scope, encoded(&endpoints::volumes::delete::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesEdit(_) => refused(scope, encoded(&endpoints::volumes::edit::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesStat(_) => refused(scope, encoded(&endpoints::volumes::stat::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesDownload(_) => refused(scope, encoded(&endpoints::volumes::download::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesUpload(_) => refused(scope, encoded(&endpoints::volumes::upload::server::response::Frame::Error(not_served()))).await,
        ClientRequest::VolumesTransfer(_) => refused(scope, encoded(&endpoints::volumes::transfer::server::response::Frame::Error(not_served()))).await,
        ClientRequest::AgentsFiletree(_) => refused(scope, encoded(&endpoints::agents::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsFiletree(_) => refused(scope, encoded(&endpoints::tools::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesFiletree(frame) => resources::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesFiletree(_) => refused(scope, encoded(&endpoints::volumes::filetree::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ResourcesGet(frame) => resources::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesTag(frame) => resources::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ResourcesUntag(frame) => resources::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::PostgresGet(_) => refused(scope, encoded(&endpoints::postgres::get::server::response::Frame::Error(not_served()))).await,
        ClientRequest::PostgresConnections(_) => refused(scope, encoded(&endpoints::postgres::connections::server::response::Frame::Error(not_served()))).await,
        ClientRequest::ToolsAdmit(frame) => tools::admit::handle(scope, frame, who, daemon).await,
        ClientRequest::ToolsUnadmit(frame) => tools::unadmit::handle(scope, frame, who, daemon).await,
        ClientRequest::Invalid(_) => refused(scope, None).await,
    }
}

/// The error every unserved request carries.
fn not_served() -> Error {
    Error(serde_json::Value::String(NOT_SERVED.to_string()))
}

/// Send the one refusal, if it encoded, and finish.
async fn refused(scope: ScopeHandle, response: Option<Vec<u8>>) {
    if let Some(bytes) = response {
        scope.send_response(&bytes).await;
    }
    scope.send_response_finish().await;
}

/// `frame` as its bytes, or `None` for the one way an error frame
/// cannot be written, its JSON failing.
fn encoded<F: Encode>(frame: &F) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    frame.encode(&mut Writer::new(&mut bytes)).ok()?;
    Some(bytes)
}
