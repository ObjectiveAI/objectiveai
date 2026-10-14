//! Every request, to its handler.

use diverge_sdk::daemon::endpoints::ClientRequest;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::{accounts, agents, dependency_tools, postgres, providers, relay, roles, standalone_tools, volumes};
use crate::daemon::Daemon;
use crate::judge::Who;

/// Read `payload` as the request that opened `scope` and hand it to
/// its handler, which answers and finishes the scope — or, when it
/// carries `relay`, to the [`relay`] conduit, which serves it on the
/// daemon named as the account named and carries the answers, the
/// member taken out of what goes on.
///
/// One arm per request of the wire, so that a request added to the
/// SDK is a request this cannot compile without. Every one of the
/// hundred and one has a handler. A payload that is no request at all is
/// finished with nothing before it, which is what the wire means by a
/// request that was not served.
pub async fn dispatch(scope: ScopeHandle, payload: &[u8], who: Who, daemon: &Arc<Daemon>) {
    let request = ClientRequest::decode(payload).unwrap_or_else(|never| match never {});
    if let Some((relay, bytes)) = relay::peel(payload) {
        return relay::serve(scope, &request, bytes, relay, who, daemon).await;
    }
    match request {
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
        ClientRequest::StandaloneToolsCreate(frame) => standalone_tools::create::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsGet(frame) => standalone_tools::get::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsEdit(frame) => standalone_tools::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsRegister(frame) => standalone_tools::register::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsAttach(frame) => standalone_tools::attach::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsDetach(frame) => standalone_tools::detach::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsDelete(frame) => standalone_tools::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsList(frame) => standalone_tools::list::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTag(frame) => standalone_tools::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsUntag(frame) => standalone_tools::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesCreate(frame) => standalone_tools::templates::create::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesGet(frame) => standalone_tools::templates::get::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesList(frame) => standalone_tools::templates::list::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesDelete(frame) => standalone_tools::templates::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesTag(frame) => standalone_tools::templates::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTemplatesUntag(frame) => standalone_tools::templates::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingAdd(frame) => providers::outgoing::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingGet(frame) => providers::outgoing::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingList(frame) => providers::outgoing::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingDelete(frame) => providers::outgoing::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingEdit(frame) => providers::outgoing::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingTag(frame) => providers::outgoing::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersOutgoingUntag(frame) => providers::outgoing::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingAdd(frame) => providers::incoming::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingGet(frame) => providers::incoming::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingList(frame) => providers::incoming::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingDelete(frame) => providers::incoming::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingEdit(frame) => providers::incoming::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingTag(frame) => providers::incoming::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersIncomingUntag(frame) => providers::incoming::untag::handle(scope, frame, who, daemon).await,
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
        ClientRequest::StandaloneToolsDownload(frame) => standalone_tools::download::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsUpload(frame) => standalone_tools::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsTransfer(frame) => standalone_tools::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesCreate(frame) => volumes::create::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesGet(frame) => volumes::get::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesList(frame) => volumes::list::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesDelete(frame) => volumes::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesEdit(frame) => volumes::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesStat(frame) => volumes::stat::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesDownload(frame) => volumes::download::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesUpload(frame) => volumes::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesTransfer(frame) => volumes::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesTag(frame) => volumes::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesUntag(frame) => volumes::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::AgentsFiletree(frame) => agents::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsFiletree(frame) => standalone_tools::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::VolumesFiletree(frame) => volumes::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::PostgresGet(frame) => postgres::get::handle(scope, frame, who, daemon).await,
        ClientRequest::PostgresList(frame) => postgres::list::handle(scope, frame, who, daemon).await,
        ClientRequest::StandaloneToolsConnect(frame) => standalone_tools::connect::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsAdd(frame) => providers::daemons::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsGet(frame) => providers::daemons::get::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsList(frame) => providers::daemons::list::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsDelete(frame) => providers::daemons::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsEdit(frame) => providers::daemons::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsTag(frame) => providers::daemons::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsUntag(frame) => providers::daemons::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsAccountsAdd(frame) => providers::daemons::accounts::add::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsAccountsEdit(frame) => providers::daemons::accounts::edit::handle(scope, frame, who, daemon).await,
        ClientRequest::ProvidersDaemonsAccountsDelete(frame) => providers::daemons::accounts::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsList(frame) => dependency_tools::list::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsGet(frame) => dependency_tools::get::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsDelete(frame) => dependency_tools::delete::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsTag(frame) => dependency_tools::tag::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsUntag(frame) => dependency_tools::untag::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsFiletree(frame) => dependency_tools::filetree::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsDownload(frame) => dependency_tools::download::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsUpload(frame) => dependency_tools::upload::handle(scope, frame, who, daemon).await,
        ClientRequest::DependencyToolsTransfer(frame) => dependency_tools::transfer::handle(scope, frame, who, daemon).await,
        ClientRequest::Invalid(_) => scope.send_response_finish().await,
    }
}
