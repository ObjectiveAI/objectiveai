//! Every request a client can open a scope with.

use std::convert::Infallible;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

use super::{accounts, agents, dependency_tools, postgres, providers, roles, standalone_tools, vault, volumes};

/// The payload of a
/// [`Request`](crate::wire::frame::client::ClientFrame::Request).
///
/// One variant per scope, in tag order, plus
/// [`Invalid`](Self::Invalid). This is the only place the tag values
/// meet: each request states its own in its own module, and nothing
/// else can see all of them at once — so this is what the table in
/// [`endpoints`](super) is a table OF.
///
/// # Decoding cannot fail
///
/// There is no error type. A payload this does not understand — an
/// unknown tag, a body that will not parse, no bytes at all — becomes
/// [`Invalid`](Self::Invalid) rather than a failure, so the daemon
/// always has a request to answer and a malformed one is a scope like
/// any other. The alternative, refusing to decode, would leave the
/// daemon holding bytes it could not name and no scope to complain in.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequest<'a> {
    /// Tag `0`. Create an agent under a name.
    AgentsCreate(agents::create::client::request::Frame),
    /// Tag `1`. Get one agent.
    AgentsGet(agents::get::client::request::Frame),
    /// Tag `2`. Delete an agent.
    AgentsDelete(agents::delete::client::request::Frame),
    /// Tag `3`. Send an agent a message.
    AgentsMessage(agents::message::client::request::Frame),
    /// Tag `4`. Read an agent's log, filtered, and perhaps kept open.
    AgentsLogs(agents::logs::client::request::Frame),
    /// Tag `5`. List the caller's agents, narrowed, and keep the list.
    AgentsList(agents::list::client::request::Frame),
    /// Tag `6`. Change an agent.
    AgentsEdit(agents::edit::client::request::Frame),
    /// Tag `7`. Put tags on an agent.
    AgentsTag(agents::tag::client::request::Frame),
    /// Tag `8`. Take tags off an agent.
    AgentsUntag(agents::untag::client::request::Frame),
    /// Tag `9`. Make a template.
    AgentsTemplatesCreate(agents::templates::create::client::request::Frame),
    /// Tag `10`. Get one template by id.
    AgentsTemplatesGet(agents::templates::get::client::request::Frame),
    /// Tag `11`. List the caller's templates, narrowed, and keep the list.
    AgentsTemplatesList(agents::templates::list::client::request::Frame),
    /// Tag `12`. Delete a template by id.
    AgentsTemplatesDelete(agents::templates::delete::client::request::Frame),
    /// Tag `13`. Put tags on a template.
    AgentsTemplatesTag(agents::templates::tag::client::request::Frame),
    /// Tag `14`. Take tags off a template.
    AgentsTemplatesUntag(agents::templates::untag::client::request::Frame),
    /// Tag `15`. Create a tool under a name.
    StandaloneToolsCreate(standalone_tools::create::client::request::Frame),
    /// Tag `16`. Get one tool.
    StandaloneToolsGet(standalone_tools::get::client::request::Frame),
    /// Tag `17`. Change a tool.
    StandaloneToolsEdit(standalone_tools::edit::client::request::Frame),
    /// Tag `18`. Register a tool another daemon holds, under a name.
    StandaloneToolsRegister(standalone_tools::register::client::request::Frame),
    /// Tag `19`. Attach a tool to one agent.
    StandaloneToolsAttachAgentInstance(standalone_tools::attach_agent_instance::client::request::Frame),
    /// Tag `20`. Detach a tool from one agent.
    StandaloneToolsDetachAgentInstance(standalone_tools::detach_agent_instance::client::request::Frame),
    /// Tag `21`. Delete a tool.
    StandaloneToolsDelete(standalone_tools::delete::client::request::Frame),
    /// Tag `22`. List the caller's tools, narrowed, and keep the list.
    StandaloneToolsList(standalone_tools::list::client::request::Frame),
    /// Tag `23`. Put tags on a tool.
    StandaloneToolsTag(standalone_tools::tag::client::request::Frame),
    /// Tag `24`. Take tags off a tool.
    StandaloneToolsUntag(standalone_tools::untag::client::request::Frame),
    /// Tag `25`. Make a tool template.
    StandaloneToolsTemplatesCreate(standalone_tools::templates::create::client::request::Frame),
    /// Tag `26`. Get one tool template by id.
    StandaloneToolsTemplatesGet(standalone_tools::templates::get::client::request::Frame),
    /// Tag `27`. List the caller's tool templates, narrowed, and keep the list.
    StandaloneToolsTemplatesList(standalone_tools::templates::list::client::request::Frame),
    /// Tag `28`. Delete a tool template by id.
    StandaloneToolsTemplatesDelete(standalone_tools::templates::delete::client::request::Frame),
    /// Tag `29`. Put tags on a tool template.
    StandaloneToolsTemplatesTag(standalone_tools::templates::tag::client::request::Frame),
    /// Tag `30`. Take tags off a tool template.
    StandaloneToolsTemplatesUntag(standalone_tools::templates::untag::client::request::Frame),
    /// Tag `31`. Add a provider to dial.
    ProvidersOutgoingAdd(providers::outgoing::add::client::request::Frame),
    /// Tag `32`. Get one outgoing provider.
    ProvidersOutgoingGet(providers::outgoing::get::client::request::Frame),
    /// Tag `33`. List the caller's outgoing providers, narrowed.
    ProvidersOutgoingList(providers::outgoing::list::client::request::Frame),
    /// Tag `34`. Forget an outgoing provider.
    ProvidersOutgoingDelete(providers::outgoing::delete::client::request::Frame),
    /// Tag `35`. Replace an outgoing provider's mode.
    ProvidersOutgoingEdit(providers::outgoing::edit::client::request::Frame),
    /// Tag `36`. Put tags on an outgoing provider.
    ProvidersOutgoingTag(providers::outgoing::tag::client::request::Frame),
    /// Tag `37`. Take tags off an outgoing provider.
    ProvidersOutgoingUntag(providers::outgoing::untag::client::request::Frame),
    /// Tag `38`. Add a credential of incoming providers.
    ProvidersIncomingAdd(providers::incoming::add::client::request::Frame),
    /// Tag `39`. Get one credential.
    ProvidersIncomingGet(providers::incoming::get::client::request::Frame),
    /// Tag `40`. List the credentials, narrowed.
    ProvidersIncomingList(providers::incoming::list::client::request::Frame),
    /// Tag `41`. Take a credential out.
    ProvidersIncomingDelete(providers::incoming::delete::client::request::Frame),
    /// Tag `42`. Replace a credential.
    ProvidersIncomingEdit(providers::incoming::edit::client::request::Frame),
    /// Tag `43`. Put tags on a credential.
    ProvidersIncomingTag(providers::incoming::tag::client::request::Frame),
    /// Tag `44`. Take tags off a credential.
    ProvidersIncomingUntag(providers::incoming::untag::client::request::Frame),
    /// Tag `45`. Create an account.
    AccountsCreate(accounts::create::client::request::Frame),
    /// Tag `46`. Get one account.
    AccountsGet(accounts::get::client::request::Frame),
    /// Tag `47`. List the accounts, narrowed, and keep the list.
    AccountsList(accounts::list::client::request::Frame),
    /// Tag `48`. Delete an account.
    AccountsDelete(accounts::delete::client::request::Frame),
    /// Tag `49`. Change an account.
    AccountsEdit(accounts::edit::client::request::Frame),
    /// Tag `50`. Put tags on an account.
    AccountsTag(accounts::tag::client::request::Frame),
    /// Tag `51`. Take tags off an account.
    AccountsUntag(accounts::untag::client::request::Frame),
    /// Tag `52`. Create a role.
    RolesCreate(roles::create::client::request::Frame),
    /// Tag `53`. Get one role.
    RolesGet(roles::get::client::request::Frame),
    /// Tag `54`. List the roles, narrowed, and keep the list.
    RolesList(roles::list::client::request::Frame),
    /// Tag `55`. Delete a role.
    RolesDelete(roles::delete::client::request::Frame),
    /// Tag `56`. Change a role.
    RolesEdit(roles::edit::client::request::Frame),
    /// Tag `57`. Put tags on a role.
    RolesTag(roles::tag::client::request::Frame),
    /// Tag `58`. Take tags off a role.
    RolesUntag(roles::untag::client::request::Frame),
    /// Tag `59`. Send the client files out of an agent's container.
    AgentsDownload(agents::download::client::request::Frame),
    /// Tag `60`. Put files into an agent's container.
    AgentsUpload(agents::upload::client::request::Frame),
    /// Tag `61`. Copy files out of an agent's container elsewhere.
    AgentsTransfer(agents::transfer::client::request::Frame),
    /// Tag `62`. Send the client files out of a tool's container.
    StandaloneToolsDownload(standalone_tools::download::client::request::Frame),
    /// Tag `63`. Put files into a tool's container.
    StandaloneToolsUpload(standalone_tools::upload::client::request::Frame),
    /// Tag `64`. Copy files out of a tool's container elsewhere.
    StandaloneToolsTransfer(standalone_tools::transfer::client::request::Frame),
    /// Tag `65`. Create a volume on a provider.
    VolumesCreate(volumes::create::client::request::Frame),
    /// Tag `66`. Get one volume.
    VolumesGet(volumes::get::client::request::Frame),
    /// Tag `67`. List the volumes, narrowed.
    VolumesList(volumes::list::client::request::Frame),
    /// Tag `68`. Delete a volume.
    VolumesDelete(volumes::delete::client::request::Frame),
    /// Tag `69`. Change a volume's size or mode.
    VolumesEdit(volumes::edit::client::request::Frame),
    /// Tag `70`. Put tags on a volume.
    VolumesTag(volumes::tag::client::request::Frame),
    /// Tag `71`. Take tags off a volume.
    VolumesUntag(volumes::untag::client::request::Frame),
    /// Tag `72`. Walk a volume for its use and its hash.
    VolumesStat(volumes::stat::client::request::Frame),
    /// Tag `73`. Send the client files out of a volume.
    VolumesDownload(volumes::download::client::request::Frame),
    /// Tag `74`. Put files into a volume.
    VolumesUpload(volumes::upload::client::request::Frame),
    /// Tag `75`. Copy files out of a volume elsewhere.
    VolumesTransfer(volumes::transfer::client::request::Frame),
    /// Tag `76`. Watch an agent's container whole.
    AgentsFiletree(agents::filetree::client::request::Frame),
    /// Tag `77`. Watch a tool's container whole.
    StandaloneToolsFiletree(standalone_tools::filetree::client::request::Frame),
    /// Tag `78`. See a volume's tree, once.
    VolumesFiletree(volumes::filetree::client::request::Frame),
    /// Tag `79`. Read which database the daemon serves.
    PostgresGet(postgres::get::client::request::Frame),
    /// Tag `80`. List the container connections open through the database.
    PostgresList(postgres::list::client::request::Frame),
    /// Tag `81`. Serve a tool to another daemon.
    StandaloneToolsConnect(standalone_tools::connect::client::request::Frame),
    /// Tag `82`. Add a daemon to connect to.
    ProvidersDaemonsAdd(providers::daemons::add::client::request::Frame),
    /// Tag `83`. Get one daemon.
    ProvidersDaemonsGet(providers::daemons::get::client::request::Frame),
    /// Tag `84`. List the caller's daemons, narrowed.
    ProvidersDaemonsList(providers::daemons::list::client::request::Frame),
    /// Tag `85`. Forget a daemon.
    ProvidersDaemonsDelete(providers::daemons::delete::client::request::Frame),
    /// Tag `86`. Replace a daemon's mode or its links.
    ProvidersDaemonsEdit(providers::daemons::edit::client::request::Frame),
    /// Tag `87`. Put tags on a daemon.
    ProvidersDaemonsTag(providers::daemons::tag::client::request::Frame),
    /// Tag `88`. Take tags off a daemon.
    ProvidersDaemonsUntag(providers::daemons::untag::client::request::Frame),
    /// Tag `89`. List the caller's dependency templates, narrowed.
    DependencyToolsList(dependency_tools::list::client::request::Frame),
    /// Tag `90`. Get one dependency template.
    DependencyToolsGet(dependency_tools::get::client::request::Frame),
    /// Tag `91`. Delete a dependency template no tool runs from.
    DependencyToolsDelete(dependency_tools::delete::client::request::Frame),
    /// Tag `92`. Put tags on a dependency template.
    DependencyToolsTag(dependency_tools::tag::client::request::Frame),
    /// Tag `93`. Take tags off a dependency template.
    DependencyToolsUntag(dependency_tools::untag::client::request::Frame),
    /// Tag `94`. Watch a running dependency tool's container whole.
    DependencyToolsFiletree(dependency_tools::filetree::client::request::Frame),
    /// Tag `95`. Download out of a running dependency tool's
    /// container.
    DependencyToolsDownload(dependency_tools::download::client::request::Frame),
    /// Tag `96`. Upload into a running dependency tool's container.
    DependencyToolsUpload(dependency_tools::upload::client::request::Frame),
    /// Tag `97`. Transfer out of a running dependency tool's
    /// container.
    DependencyToolsTransfer(dependency_tools::transfer::client::request::Frame),
    /// Tag `98`. Add an account to a daemon record.
    ProvidersDaemonsAccountsAdd(providers::daemons::accounts::add::client::request::Frame),
    /// Tag `99`. Replace an account's mode on a daemon record.
    ProvidersDaemonsAccountsEdit(providers::daemons::accounts::edit::client::request::Frame),
    /// Tag `100`. Forget an account on a daemon record.
    ProvidersDaemonsAccountsDelete(providers::daemons::accounts::delete::client::request::Frame),
    /// Tag `101`. Attach a tool to every agent of a template.
    StandaloneToolsAttachAgentTemplate(standalone_tools::attach_agent_template::client::request::Frame),
    /// Tag `102`. Detach a tool from every agent of a template.
    StandaloneToolsDetachAgentTemplate(standalone_tools::detach_agent_template::client::request::Frame),
    /// Something this version cannot read, kept as it arrived.
    ///
    /// Tag `103`. Get one vault key, with its value.
    VaultGet(vault::get::client::request::Frame),
    /// Tag `104`. Insert a value into a vault key: the key made,
    /// the value appended, or a slot's value replaced.
    VaultInsert(vault::insert::client::request::Frame),
    /// Tag `105`. Delete a vault key nothing binds.
    VaultDelete(vault::delete::client::request::Frame),
    /// Tag `106`. List the vault's keys, narrowed, and keep the list.
    VaultList(vault::list::client::request::Frame),
    /// Tag `107`. Put tags on a vault key.
    VaultTag(vault::tag::client::request::Frame),
    /// Tag `108`. Take tags off a vault key.
    VaultUntag(vault::untag::client::request::Frame),
    /// No tag of its own. It is not a request a client sends — it is
    /// what a request BECOMES when the daemon cannot make one of the
    /// others out of it, so it carries the payload verbatim and
    /// encodes back to exactly those bytes. The daemon finishes the
    /// scope over it with nothing in front: a finish that no response
    /// precedes is already what the wire means by a request that
    /// could not be served, and it names no endpoint's error
    /// vocabulary, which an invalid request has none of.
    Invalid(&'a [u8]),
}

impl Encode for ClientRequest<'_> {
    /// The ordinary JSON failure: every request is JSON after its
    /// tag.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            ClientRequest::AgentsCreate(frame) => frame.encode(out),
            ClientRequest::AgentsGet(frame) => frame.encode(out),
            ClientRequest::AgentsDelete(frame) => frame.encode(out),
            ClientRequest::AgentsMessage(frame) => frame.encode(out),
            ClientRequest::AgentsLogs(frame) => frame.encode(out),
            ClientRequest::AgentsList(frame) => frame.encode(out),
            ClientRequest::AgentsEdit(frame) => frame.encode(out),
            ClientRequest::AgentsTag(frame) => frame.encode(out),
            ClientRequest::AgentsUntag(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesCreate(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesGet(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesList(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesDelete(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesTag(frame) => frame.encode(out),
            ClientRequest::AgentsTemplatesUntag(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsCreate(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsGet(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsEdit(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsRegister(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsAttachAgentInstance(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsDetachAgentInstance(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsDelete(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsList(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTag(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsUntag(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesCreate(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesGet(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesList(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesDelete(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesTag(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTemplatesUntag(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingList(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingTag(frame) => frame.encode(out),
            ClientRequest::ProvidersOutgoingUntag(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingGet(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingList(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingTag(frame) => frame.encode(out),
            ClientRequest::ProvidersIncomingUntag(frame) => frame.encode(out),
            ClientRequest::AccountsCreate(frame) => frame.encode(out),
            ClientRequest::AccountsGet(frame) => frame.encode(out),
            ClientRequest::AccountsList(frame) => frame.encode(out),
            ClientRequest::AccountsDelete(frame) => frame.encode(out),
            ClientRequest::AccountsEdit(frame) => frame.encode(out),
            ClientRequest::AccountsTag(frame) => frame.encode(out),
            ClientRequest::AccountsUntag(frame) => frame.encode(out),
            ClientRequest::RolesCreate(frame) => frame.encode(out),
            ClientRequest::RolesGet(frame) => frame.encode(out),
            ClientRequest::RolesList(frame) => frame.encode(out),
            ClientRequest::RolesDelete(frame) => frame.encode(out),
            ClientRequest::RolesEdit(frame) => frame.encode(out),
            ClientRequest::RolesTag(frame) => frame.encode(out),
            ClientRequest::RolesUntag(frame) => frame.encode(out),
            ClientRequest::AgentsDownload(frame) => frame.encode(out),
            ClientRequest::AgentsUpload(frame) => frame.encode(out),
            ClientRequest::AgentsTransfer(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsDownload(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsUpload(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsTransfer(frame) => frame.encode(out),
            ClientRequest::VolumesCreate(frame) => frame.encode(out),
            ClientRequest::VolumesGet(frame) => frame.encode(out),
            ClientRequest::VolumesList(frame) => frame.encode(out),
            ClientRequest::VolumesDelete(frame) => frame.encode(out),
            ClientRequest::VolumesEdit(frame) => frame.encode(out),
            ClientRequest::VolumesTag(frame) => frame.encode(out),
            ClientRequest::VolumesUntag(frame) => frame.encode(out),
            ClientRequest::VolumesStat(frame) => frame.encode(out),
            ClientRequest::VolumesDownload(frame) => frame.encode(out),
            ClientRequest::VolumesUpload(frame) => frame.encode(out),
            ClientRequest::VolumesTransfer(frame) => frame.encode(out),
            ClientRequest::AgentsFiletree(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsFiletree(frame) => frame.encode(out),
            ClientRequest::VolumesFiletree(frame) => frame.encode(out),
            ClientRequest::PostgresGet(frame) => frame.encode(out),
            ClientRequest::PostgresList(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsConnect(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsGet(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsList(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsDelete(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsTag(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsUntag(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsAccountsAdd(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsAccountsEdit(frame) => frame.encode(out),
            ClientRequest::ProvidersDaemonsAccountsDelete(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsAttachAgentTemplate(frame) => frame.encode(out),
            ClientRequest::StandaloneToolsDetachAgentTemplate(frame) => frame.encode(out),
            ClientRequest::DependencyToolsList(frame) => frame.encode(out),
            ClientRequest::DependencyToolsGet(frame) => frame.encode(out),
            ClientRequest::DependencyToolsDelete(frame) => frame.encode(out),
            ClientRequest::DependencyToolsTag(frame) => frame.encode(out),
            ClientRequest::DependencyToolsUntag(frame) => frame.encode(out),
            ClientRequest::DependencyToolsFiletree(frame) => frame.encode(out),
            ClientRequest::DependencyToolsDownload(frame) => frame.encode(out),
            ClientRequest::DependencyToolsUpload(frame) => frame.encode(out),
            ClientRequest::DependencyToolsTransfer(frame) => frame.encode(out),
            ClientRequest::VaultGet(frame) => frame.encode(out),
            ClientRequest::VaultInsert(frame) => frame.encode(out),
            ClientRequest::VaultDelete(frame) => frame.encode(out),
            ClientRequest::VaultList(frame) => frame.encode(out),
            ClientRequest::VaultTag(frame) => frame.encode(out),
            ClientRequest::VaultUntag(frame) => frame.encode(out),
            ClientRequest::Invalid(bytes) => {
                out.extend_from_slice(bytes);
                Ok(())
            }
        }
    }
}

/// Dispatch on the tag, and fall back to
/// [`Invalid`](ClientRequest::Invalid) rather than refusing.
///
/// Each request is handed the WHOLE payload, tag included, because
/// each checks its own tag — this reads the byte to choose a decoder
/// and does not strip it.
impl<'a> Decode<'a> for ClientRequest<'a> {
    /// [`Infallible`]: a payload that cannot be read is a request all
    /// the same.
    type Error = Infallible;

    fn decode(bytes: &'a [u8]) -> Result<Self, Infallible> {
        let Some(tag) = bytes.first() else {
            return Ok(ClientRequest::Invalid(bytes));
        };
        let request = match *tag {
            0 => agents::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsCreate)
                .ok(),
            1 => agents::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsGet)
                .ok(),
            2 => agents::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDelete)
                .ok(),
            3 => agents::message::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsMessage)
                .ok(),
            4 => agents::logs::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsLogs)
                .ok(),
            5 => agents::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsList)
                .ok(),
            6 => agents::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsEdit)
                .ok(),
            7 => agents::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTag)
                .ok(),
            8 => agents::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsUntag)
                .ok(),
            9 => agents::templates::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesCreate)
                .ok(),
            10 => agents::templates::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesGet)
                .ok(),
            11 => agents::templates::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesList)
                .ok(),
            12 => agents::templates::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesDelete)
                .ok(),
            13 => agents::templates::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesTag)
                .ok(),
            14 => agents::templates::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTemplatesUntag)
                .ok(),
            15 => standalone_tools::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsCreate)
                .ok(),
            16 => standalone_tools::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsGet)
                .ok(),
            17 => standalone_tools::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsEdit)
                .ok(),
            18 => standalone_tools::register::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsRegister)
                .ok(),
            19 => standalone_tools::attach_agent_instance::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsAttachAgentInstance)
                .ok(),
            20 => standalone_tools::detach_agent_instance::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsDetachAgentInstance)
                .ok(),
            21 => standalone_tools::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsDelete)
                .ok(),
            22 => standalone_tools::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsList)
                .ok(),
            23 => standalone_tools::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTag)
                .ok(),
            24 => standalone_tools::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsUntag)
                .ok(),
            25 => standalone_tools::templates::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesCreate)
                .ok(),
            26 => standalone_tools::templates::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesGet)
                .ok(),
            27 => standalone_tools::templates::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesList)
                .ok(),
            28 => standalone_tools::templates::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesDelete)
                .ok(),
            29 => standalone_tools::templates::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesTag)
                .ok(),
            30 => standalone_tools::templates::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTemplatesUntag)
                .ok(),
            31 => providers::outgoing::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingAdd)
                .ok(),
            32 => providers::outgoing::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingGet)
                .ok(),
            33 => providers::outgoing::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingList)
                .ok(),
            34 => providers::outgoing::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingDelete)
                .ok(),
            35 => providers::outgoing::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingEdit)
                .ok(),
            36 => providers::outgoing::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingTag)
                .ok(),
            37 => providers::outgoing::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersOutgoingUntag)
                .ok(),
            38 => providers::incoming::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingAdd)
                .ok(),
            39 => providers::incoming::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingGet)
                .ok(),
            40 => providers::incoming::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingList)
                .ok(),
            41 => providers::incoming::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingDelete)
                .ok(),
            42 => providers::incoming::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingEdit)
                .ok(),
            43 => providers::incoming::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingTag)
                .ok(),
            44 => providers::incoming::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersIncomingUntag)
                .ok(),
            45 => accounts::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsCreate)
                .ok(),
            46 => accounts::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsGet)
                .ok(),
            47 => accounts::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsList)
                .ok(),
            48 => accounts::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsDelete)
                .ok(),
            49 => accounts::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsEdit)
                .ok(),
            50 => accounts::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsTag)
                .ok(),
            51 => accounts::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::AccountsUntag)
                .ok(),
            52 => roles::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesCreate)
                .ok(),
            53 => roles::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesGet)
                .ok(),
            54 => roles::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesList)
                .ok(),
            55 => roles::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesDelete)
                .ok(),
            56 => roles::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesEdit)
                .ok(),
            57 => roles::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesTag)
                .ok(),
            58 => roles::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::RolesUntag)
                .ok(),
            59 => agents::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsDownload)
                .ok(),
            60 => agents::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsUpload)
                .ok(),
            61 => agents::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsTransfer)
                .ok(),
            62 => standalone_tools::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsDownload)
                .ok(),
            63 => standalone_tools::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsUpload)
                .ok(),
            64 => standalone_tools::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsTransfer)
                .ok(),
            65 => volumes::create::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesCreate)
                .ok(),
            66 => volumes::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesGet)
                .ok(),
            67 => volumes::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesList)
                .ok(),
            68 => volumes::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDelete)
                .ok(),
            69 => volumes::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesEdit)
                .ok(),
            70 => volumes::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesTag)
                .ok(),
            71 => volumes::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesUntag)
                .ok(),
            72 => volumes::stat::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesStat)
                .ok(),
            73 => volumes::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesDownload)
                .ok(),
            74 => volumes::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesUpload)
                .ok(),
            75 => volumes::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesTransfer)
                .ok(),
            76 => agents::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::AgentsFiletree)
                .ok(),
            77 => standalone_tools::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsFiletree)
                .ok(),
            78 => volumes::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::VolumesFiletree)
                .ok(),
            79 => postgres::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::PostgresGet)
                .ok(),
            80 => postgres::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::PostgresList)
                .ok(),
            81 => standalone_tools::connect::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsConnect)
                .ok(),
            82 => providers::daemons::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsAdd)
                .ok(),
            83 => providers::daemons::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsGet)
                .ok(),
            84 => providers::daemons::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsList)
                .ok(),
            85 => providers::daemons::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsDelete)
                .ok(),
            86 => providers::daemons::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsEdit)
                .ok(),
            87 => providers::daemons::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsTag)
                .ok(),
            88 => providers::daemons::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsUntag)
                .ok(),
            89 => dependency_tools::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsList)
                .ok(),
            90 => dependency_tools::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsGet)
                .ok(),
            91 => dependency_tools::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsDelete)
                .ok(),
            92 => dependency_tools::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsTag)
                .ok(),
            93 => dependency_tools::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsUntag)
                .ok(),
            94 => dependency_tools::filetree::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsFiletree)
                .ok(),
            95 => dependency_tools::download::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsDownload)
                .ok(),
            96 => dependency_tools::upload::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsUpload)
                .ok(),
            97 => dependency_tools::transfer::client::request::Frame::decode(bytes)
                .map(ClientRequest::DependencyToolsTransfer)
                .ok(),
            98 => providers::daemons::accounts::add::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsAccountsAdd)
                .ok(),
            99 => providers::daemons::accounts::edit::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsAccountsEdit)
                .ok(),
            100 => providers::daemons::accounts::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::ProvidersDaemonsAccountsDelete)
                .ok(),
            101 => standalone_tools::attach_agent_template::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsAttachAgentTemplate)
                .ok(),
            102 => standalone_tools::detach_agent_template::client::request::Frame::decode(bytes)
                .map(ClientRequest::StandaloneToolsDetachAgentTemplate)
                .ok(),
            103 => vault::get::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultGet)
                .ok(),
            104 => vault::insert::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultInsert)
                .ok(),
            105 => vault::delete::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultDelete)
                .ok(),
            106 => vault::list::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultList)
                .ok(),
            107 => vault::tag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultTag)
                .ok(),
            108 => vault::untag::client::request::Frame::decode(bytes)
                .map(ClientRequest::VaultUntag)
                .ok(),
            _ => None,
        };
        Ok(request.unwrap_or(ClientRequest::Invalid(bytes)))
    }
}

impl fmt::Display for ClientRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientRequest::AgentsCreate(_) => f.write_str("agents create"),
            ClientRequest::AgentsGet(_) => f.write_str("agents get"),
            ClientRequest::AgentsDelete(_) => f.write_str("agents delete"),
            ClientRequest::AgentsMessage(_) => f.write_str("agents message"),
            ClientRequest::AgentsLogs(_) => f.write_str("agents logs"),
            ClientRequest::AgentsList(_) => f.write_str("agents list"),
            ClientRequest::AgentsEdit(_) => f.write_str("agents edit"),
            ClientRequest::AgentsTag(_) => f.write_str("agents tag"),
            ClientRequest::AgentsUntag(_) => f.write_str("agents untag"),
            ClientRequest::AgentsTemplatesCreate(_) => f.write_str("agents templates create"),
            ClientRequest::AgentsTemplatesGet(_) => f.write_str("agents templates get"),
            ClientRequest::AgentsTemplatesList(_) => f.write_str("agents templates list"),
            ClientRequest::AgentsTemplatesDelete(_) => f.write_str("agents templates delete"),
            ClientRequest::AgentsTemplatesTag(_) => f.write_str("agents templates tag"),
            ClientRequest::AgentsTemplatesUntag(_) => f.write_str("agents templates untag"),
            ClientRequest::StandaloneToolsCreate(_) => f.write_str("standalone tools create"),
            ClientRequest::StandaloneToolsGet(_) => f.write_str("standalone tools get"),
            ClientRequest::StandaloneToolsEdit(_) => f.write_str("standalone tools edit"),
            ClientRequest::StandaloneToolsRegister(_) => f.write_str("standalone tools register"),
            ClientRequest::StandaloneToolsAttachAgentInstance(_) => f.write_str("standalone tools attach agent instance"),
            ClientRequest::StandaloneToolsDetachAgentInstance(_) => f.write_str("standalone tools detach agent instance"),
            ClientRequest::StandaloneToolsDelete(_) => f.write_str("standalone tools delete"),
            ClientRequest::StandaloneToolsList(_) => f.write_str("standalone tools list"),
            ClientRequest::StandaloneToolsTag(_) => f.write_str("standalone tools tag"),
            ClientRequest::StandaloneToolsUntag(_) => f.write_str("standalone tools untag"),
            ClientRequest::StandaloneToolsTemplatesCreate(_) => f.write_str("standalone tools templates create"),
            ClientRequest::StandaloneToolsTemplatesGet(_) => f.write_str("standalone tools templates get"),
            ClientRequest::StandaloneToolsTemplatesList(_) => f.write_str("standalone tools templates list"),
            ClientRequest::StandaloneToolsTemplatesDelete(_) => f.write_str("standalone tools templates delete"),
            ClientRequest::StandaloneToolsTemplatesTag(_) => f.write_str("standalone tools templates tag"),
            ClientRequest::StandaloneToolsTemplatesUntag(_) => f.write_str("standalone tools templates untag"),
            ClientRequest::ProvidersOutgoingAdd(_) => f.write_str("providers outgoing add"),
            ClientRequest::ProvidersOutgoingGet(_) => f.write_str("providers outgoing get"),
            ClientRequest::ProvidersOutgoingList(_) => f.write_str("providers outgoing list"),
            ClientRequest::ProvidersOutgoingDelete(_) => f.write_str("providers outgoing delete"),
            ClientRequest::ProvidersOutgoingEdit(_) => f.write_str("providers outgoing edit"),
            ClientRequest::ProvidersOutgoingTag(_) => f.write_str("providers outgoing tag"),
            ClientRequest::ProvidersOutgoingUntag(_) => f.write_str("providers outgoing untag"),
            ClientRequest::ProvidersIncomingAdd(_) => f.write_str("providers incoming add"),
            ClientRequest::ProvidersIncomingGet(_) => f.write_str("providers incoming get"),
            ClientRequest::ProvidersIncomingList(_) => f.write_str("providers incoming list"),
            ClientRequest::ProvidersIncomingDelete(_) => f.write_str("providers incoming delete"),
            ClientRequest::ProvidersIncomingEdit(_) => f.write_str("providers incoming edit"),
            ClientRequest::ProvidersIncomingTag(_) => f.write_str("providers incoming tag"),
            ClientRequest::ProvidersIncomingUntag(_) => f.write_str("providers incoming untag"),
            ClientRequest::AccountsCreate(_) => f.write_str("accounts create"),
            ClientRequest::AccountsGet(_) => f.write_str("accounts get"),
            ClientRequest::AccountsList(_) => f.write_str("accounts list"),
            ClientRequest::AccountsDelete(_) => f.write_str("accounts delete"),
            ClientRequest::AccountsEdit(_) => f.write_str("accounts edit"),
            ClientRequest::AccountsTag(_) => f.write_str("accounts tag"),
            ClientRequest::AccountsUntag(_) => f.write_str("accounts untag"),
            ClientRequest::RolesCreate(_) => f.write_str("roles create"),
            ClientRequest::RolesGet(_) => f.write_str("roles get"),
            ClientRequest::RolesList(_) => f.write_str("roles list"),
            ClientRequest::RolesDelete(_) => f.write_str("roles delete"),
            ClientRequest::RolesEdit(_) => f.write_str("roles edit"),
            ClientRequest::RolesTag(_) => f.write_str("roles tag"),
            ClientRequest::RolesUntag(_) => f.write_str("roles untag"),
            ClientRequest::AgentsDownload(_) => f.write_str("agents download"),
            ClientRequest::AgentsUpload(_) => f.write_str("agents upload"),
            ClientRequest::AgentsTransfer(_) => f.write_str("agents transfer"),
            ClientRequest::StandaloneToolsDownload(_) => f.write_str("standalone tools download"),
            ClientRequest::StandaloneToolsUpload(_) => f.write_str("standalone tools upload"),
            ClientRequest::StandaloneToolsTransfer(_) => f.write_str("standalone tools transfer"),
            ClientRequest::VolumesCreate(_) => f.write_str("volumes create"),
            ClientRequest::VolumesGet(_) => f.write_str("volumes get"),
            ClientRequest::VolumesList(_) => f.write_str("volumes list"),
            ClientRequest::VolumesDelete(_) => f.write_str("volumes delete"),
            ClientRequest::VolumesEdit(_) => f.write_str("volumes edit"),
            ClientRequest::VolumesTag(_) => f.write_str("volumes tag"),
            ClientRequest::VolumesUntag(_) => f.write_str("volumes untag"),
            ClientRequest::VolumesStat(_) => f.write_str("volumes stat"),
            ClientRequest::VolumesDownload(_) => f.write_str("volumes download"),
            ClientRequest::VolumesUpload(_) => f.write_str("volumes upload"),
            ClientRequest::VolumesTransfer(_) => f.write_str("volumes transfer"),
            ClientRequest::AgentsFiletree(_) => f.write_str("agents filetree"),
            ClientRequest::StandaloneToolsFiletree(_) => f.write_str("standalone tools filetree"),
            ClientRequest::VolumesFiletree(_) => f.write_str("volumes filetree"),
            ClientRequest::PostgresGet(_) => f.write_str("postgres get"),
            ClientRequest::PostgresList(_) => f.write_str("postgres list"),
            ClientRequest::StandaloneToolsConnect(_) => f.write_str("standalone tools connect"),
            ClientRequest::ProvidersDaemonsAdd(_) => f.write_str("providers daemons add"),
            ClientRequest::ProvidersDaemonsGet(_) => f.write_str("providers daemons get"),
            ClientRequest::ProvidersDaemonsList(_) => f.write_str("providers daemons list"),
            ClientRequest::ProvidersDaemonsDelete(_) => f.write_str("providers daemons delete"),
            ClientRequest::ProvidersDaemonsEdit(_) => f.write_str("providers daemons edit"),
            ClientRequest::ProvidersDaemonsTag(_) => f.write_str("providers daemons tag"),
            ClientRequest::ProvidersDaemonsUntag(_) => f.write_str("providers daemons untag"),
            ClientRequest::ProvidersDaemonsAccountsAdd(_) => f.write_str("providers daemons accounts add"),
            ClientRequest::ProvidersDaemonsAccountsEdit(_) => f.write_str("providers daemons accounts edit"),
            ClientRequest::ProvidersDaemonsAccountsDelete(_) => f.write_str("providers daemons accounts delete"),
            ClientRequest::StandaloneToolsAttachAgentTemplate(_) => f.write_str("standalone tools attach agent template"),
            ClientRequest::StandaloneToolsDetachAgentTemplate(_) => f.write_str("standalone tools detach agent template"),
            ClientRequest::DependencyToolsList(_) => f.write_str("dependency tools list"),
            ClientRequest::DependencyToolsGet(_) => f.write_str("dependency tools get"),
            ClientRequest::DependencyToolsDelete(_) => f.write_str("dependency tools delete"),
            ClientRequest::DependencyToolsTag(_) => f.write_str("dependency tools tag"),
            ClientRequest::DependencyToolsUntag(_) => f.write_str("dependency tools untag"),
            ClientRequest::DependencyToolsFiletree(_) => f.write_str("dependency tools filetree"),
            ClientRequest::DependencyToolsDownload(_) => f.write_str("dependency tools download"),
            ClientRequest::DependencyToolsUpload(_) => f.write_str("dependency tools upload"),
            ClientRequest::DependencyToolsTransfer(_) => f.write_str("dependency tools transfer"),
            ClientRequest::VaultGet(_) => f.write_str("vault get"),
            ClientRequest::VaultInsert(_) => f.write_str("vault insert"),
            ClientRequest::VaultDelete(_) => f.write_str("vault delete"),
            ClientRequest::VaultList(_) => f.write_str("vault list"),
            ClientRequest::VaultTag(_) => f.write_str("vault tag"),
            ClientRequest::VaultUntag(_) => f.write_str("vault untag"),
            ClientRequest::Invalid(_) => f.write_str("an invalid request"),
        }
    }
}

/// The tag of the request `path` names: the request's name as a
/// client writes it on a command line, the family and the endpoint
/// joined by `/` — `agents/create`, `standalone_tools/templates/list`,
/// `providers/daemons/add` — which is the module path of the
/// endpoint, slashed. `None` for a name that is no request.
pub fn tag(path: &str) -> Option<u8> {
    match path {
        "agents/create" => Some(0),
        "agents/get" => Some(1),
        "agents/delete" => Some(2),
        "agents/message" => Some(3),
        "agents/logs" => Some(4),
        "agents/list" => Some(5),
        "agents/edit" => Some(6),
        "agents/tag" => Some(7),
        "agents/untag" => Some(8),
        "agents/templates/create" => Some(9),
        "agents/templates/get" => Some(10),
        "agents/templates/list" => Some(11),
        "agents/templates/delete" => Some(12),
        "agents/templates/tag" => Some(13),
        "agents/templates/untag" => Some(14),
        "standalone_tools/create" => Some(15),
        "standalone_tools/get" => Some(16),
        "standalone_tools/edit" => Some(17),
        "standalone_tools/register" => Some(18),
        "standalone_tools/attach_agent_instance" => Some(19),
        "standalone_tools/detach_agent_instance" => Some(20),
        "standalone_tools/delete" => Some(21),
        "standalone_tools/list" => Some(22),
        "standalone_tools/tag" => Some(23),
        "standalone_tools/untag" => Some(24),
        "standalone_tools/templates/create" => Some(25),
        "standalone_tools/templates/get" => Some(26),
        "standalone_tools/templates/list" => Some(27),
        "standalone_tools/templates/delete" => Some(28),
        "standalone_tools/templates/tag" => Some(29),
        "standalone_tools/templates/untag" => Some(30),
        "providers/outgoing/add" => Some(31),
        "providers/outgoing/get" => Some(32),
        "providers/outgoing/list" => Some(33),
        "providers/outgoing/delete" => Some(34),
        "providers/outgoing/edit" => Some(35),
        "providers/outgoing/tag" => Some(36),
        "providers/outgoing/untag" => Some(37),
        "providers/incoming/add" => Some(38),
        "providers/incoming/get" => Some(39),
        "providers/incoming/list" => Some(40),
        "providers/incoming/delete" => Some(41),
        "providers/incoming/edit" => Some(42),
        "providers/incoming/tag" => Some(43),
        "providers/incoming/untag" => Some(44),
        "accounts/create" => Some(45),
        "accounts/get" => Some(46),
        "accounts/list" => Some(47),
        "accounts/delete" => Some(48),
        "accounts/edit" => Some(49),
        "accounts/tag" => Some(50),
        "accounts/untag" => Some(51),
        "roles/create" => Some(52),
        "roles/get" => Some(53),
        "roles/list" => Some(54),
        "roles/delete" => Some(55),
        "roles/edit" => Some(56),
        "roles/tag" => Some(57),
        "roles/untag" => Some(58),
        "agents/download" => Some(59),
        "agents/upload" => Some(60),
        "agents/transfer" => Some(61),
        "standalone_tools/download" => Some(62),
        "standalone_tools/upload" => Some(63),
        "standalone_tools/transfer" => Some(64),
        "volumes/create" => Some(65),
        "volumes/get" => Some(66),
        "volumes/list" => Some(67),
        "volumes/delete" => Some(68),
        "volumes/edit" => Some(69),
        "volumes/tag" => Some(70),
        "volumes/untag" => Some(71),
        "volumes/stat" => Some(72),
        "volumes/download" => Some(73),
        "volumes/upload" => Some(74),
        "volumes/transfer" => Some(75),
        "agents/filetree" => Some(76),
        "standalone_tools/filetree" => Some(77),
        "volumes/filetree" => Some(78),
        "postgres/get" => Some(79),
        "postgres/list" => Some(80),
        "standalone_tools/connect" => Some(81),
        "providers/daemons/add" => Some(82),
        "providers/daemons/get" => Some(83),
        "providers/daemons/list" => Some(84),
        "providers/daemons/delete" => Some(85),
        "providers/daemons/edit" => Some(86),
        "providers/daemons/tag" => Some(87),
        "providers/daemons/untag" => Some(88),
        "dependency_tools/list" => Some(89),
        "dependency_tools/get" => Some(90),
        "dependency_tools/delete" => Some(91),
        "dependency_tools/tag" => Some(92),
        "dependency_tools/untag" => Some(93),
        "dependency_tools/filetree" => Some(94),
        "dependency_tools/download" => Some(95),
        "dependency_tools/upload" => Some(96),
        "dependency_tools/transfer" => Some(97),
        "providers/daemons/accounts/add" => Some(98),
        "providers/daemons/accounts/edit" => Some(99),
        "providers/daemons/accounts/delete" => Some(100),
        "standalone_tools/attach_agent_template" => Some(101),
        "standalone_tools/detach_agent_template" => Some(102),
        "vault/get" => Some(103),
        "vault/insert" => Some(104),
        "vault/delete" => Some(105),
        "vault/list" => Some(106),
        "vault/tag" => Some(107),
        "vault/untag" => Some(108),
        _ => None,
    }
}

impl ClientRequest<'_> {
    /// The request's name, as [`tag`] reads it; an invalid request has
    /// none and is the empty string.
    pub fn path(&self) -> &'static str {
        match self {
            ClientRequest::AgentsCreate(_) => "agents/create",
            ClientRequest::AgentsGet(_) => "agents/get",
            ClientRequest::AgentsDelete(_) => "agents/delete",
            ClientRequest::AgentsMessage(_) => "agents/message",
            ClientRequest::AgentsLogs(_) => "agents/logs",
            ClientRequest::AgentsList(_) => "agents/list",
            ClientRequest::AgentsEdit(_) => "agents/edit",
            ClientRequest::AgentsTag(_) => "agents/tag",
            ClientRequest::AgentsUntag(_) => "agents/untag",
            ClientRequest::AgentsTemplatesCreate(_) => "agents/templates/create",
            ClientRequest::AgentsTemplatesGet(_) => "agents/templates/get",
            ClientRequest::AgentsTemplatesList(_) => "agents/templates/list",
            ClientRequest::AgentsTemplatesDelete(_) => "agents/templates/delete",
            ClientRequest::AgentsTemplatesTag(_) => "agents/templates/tag",
            ClientRequest::AgentsTemplatesUntag(_) => "agents/templates/untag",
            ClientRequest::StandaloneToolsCreate(_) => "standalone_tools/create",
            ClientRequest::StandaloneToolsGet(_) => "standalone_tools/get",
            ClientRequest::StandaloneToolsEdit(_) => "standalone_tools/edit",
            ClientRequest::StandaloneToolsRegister(_) => "standalone_tools/register",
            ClientRequest::StandaloneToolsAttachAgentInstance(_) => "standalone_tools/attach_agent_instance",
            ClientRequest::StandaloneToolsDetachAgentInstance(_) => "standalone_tools/detach_agent_instance",
            ClientRequest::StandaloneToolsDelete(_) => "standalone_tools/delete",
            ClientRequest::StandaloneToolsList(_) => "standalone_tools/list",
            ClientRequest::StandaloneToolsTag(_) => "standalone_tools/tag",
            ClientRequest::StandaloneToolsUntag(_) => "standalone_tools/untag",
            ClientRequest::StandaloneToolsTemplatesCreate(_) => "standalone_tools/templates/create",
            ClientRequest::StandaloneToolsTemplatesGet(_) => "standalone_tools/templates/get",
            ClientRequest::StandaloneToolsTemplatesList(_) => "standalone_tools/templates/list",
            ClientRequest::StandaloneToolsTemplatesDelete(_) => "standalone_tools/templates/delete",
            ClientRequest::StandaloneToolsTemplatesTag(_) => "standalone_tools/templates/tag",
            ClientRequest::StandaloneToolsTemplatesUntag(_) => "standalone_tools/templates/untag",
            ClientRequest::ProvidersOutgoingAdd(_) => "providers/outgoing/add",
            ClientRequest::ProvidersOutgoingGet(_) => "providers/outgoing/get",
            ClientRequest::ProvidersOutgoingList(_) => "providers/outgoing/list",
            ClientRequest::ProvidersOutgoingDelete(_) => "providers/outgoing/delete",
            ClientRequest::ProvidersOutgoingEdit(_) => "providers/outgoing/edit",
            ClientRequest::ProvidersOutgoingTag(_) => "providers/outgoing/tag",
            ClientRequest::ProvidersOutgoingUntag(_) => "providers/outgoing/untag",
            ClientRequest::ProvidersIncomingAdd(_) => "providers/incoming/add",
            ClientRequest::ProvidersIncomingGet(_) => "providers/incoming/get",
            ClientRequest::ProvidersIncomingList(_) => "providers/incoming/list",
            ClientRequest::ProvidersIncomingDelete(_) => "providers/incoming/delete",
            ClientRequest::ProvidersIncomingEdit(_) => "providers/incoming/edit",
            ClientRequest::ProvidersIncomingTag(_) => "providers/incoming/tag",
            ClientRequest::ProvidersIncomingUntag(_) => "providers/incoming/untag",
            ClientRequest::AccountsCreate(_) => "accounts/create",
            ClientRequest::AccountsGet(_) => "accounts/get",
            ClientRequest::AccountsList(_) => "accounts/list",
            ClientRequest::AccountsDelete(_) => "accounts/delete",
            ClientRequest::AccountsEdit(_) => "accounts/edit",
            ClientRequest::AccountsTag(_) => "accounts/tag",
            ClientRequest::AccountsUntag(_) => "accounts/untag",
            ClientRequest::RolesCreate(_) => "roles/create",
            ClientRequest::RolesGet(_) => "roles/get",
            ClientRequest::RolesList(_) => "roles/list",
            ClientRequest::RolesDelete(_) => "roles/delete",
            ClientRequest::RolesEdit(_) => "roles/edit",
            ClientRequest::RolesTag(_) => "roles/tag",
            ClientRequest::RolesUntag(_) => "roles/untag",
            ClientRequest::AgentsDownload(_) => "agents/download",
            ClientRequest::AgentsUpload(_) => "agents/upload",
            ClientRequest::AgentsTransfer(_) => "agents/transfer",
            ClientRequest::StandaloneToolsDownload(_) => "standalone_tools/download",
            ClientRequest::StandaloneToolsUpload(_) => "standalone_tools/upload",
            ClientRequest::StandaloneToolsTransfer(_) => "standalone_tools/transfer",
            ClientRequest::VolumesCreate(_) => "volumes/create",
            ClientRequest::VolumesGet(_) => "volumes/get",
            ClientRequest::VolumesList(_) => "volumes/list",
            ClientRequest::VolumesDelete(_) => "volumes/delete",
            ClientRequest::VolumesEdit(_) => "volumes/edit",
            ClientRequest::VolumesTag(_) => "volumes/tag",
            ClientRequest::VolumesUntag(_) => "volumes/untag",
            ClientRequest::VolumesStat(_) => "volumes/stat",
            ClientRequest::VolumesDownload(_) => "volumes/download",
            ClientRequest::VolumesUpload(_) => "volumes/upload",
            ClientRequest::VolumesTransfer(_) => "volumes/transfer",
            ClientRequest::AgentsFiletree(_) => "agents/filetree",
            ClientRequest::StandaloneToolsFiletree(_) => "standalone_tools/filetree",
            ClientRequest::VolumesFiletree(_) => "volumes/filetree",
            ClientRequest::PostgresGet(_) => "postgres/get",
            ClientRequest::PostgresList(_) => "postgres/list",
            ClientRequest::StandaloneToolsConnect(_) => "standalone_tools/connect",
            ClientRequest::ProvidersDaemonsAdd(_) => "providers/daemons/add",
            ClientRequest::ProvidersDaemonsGet(_) => "providers/daemons/get",
            ClientRequest::ProvidersDaemonsList(_) => "providers/daemons/list",
            ClientRequest::ProvidersDaemonsDelete(_) => "providers/daemons/delete",
            ClientRequest::ProvidersDaemonsEdit(_) => "providers/daemons/edit",
            ClientRequest::ProvidersDaemonsTag(_) => "providers/daemons/tag",
            ClientRequest::ProvidersDaemonsUntag(_) => "providers/daemons/untag",
            ClientRequest::ProvidersDaemonsAccountsAdd(_) => "providers/daemons/accounts/add",
            ClientRequest::ProvidersDaemonsAccountsEdit(_) => "providers/daemons/accounts/edit",
            ClientRequest::ProvidersDaemonsAccountsDelete(_) => "providers/daemons/accounts/delete",
            ClientRequest::StandaloneToolsAttachAgentTemplate(_) => "standalone_tools/attach_agent_template",
            ClientRequest::StandaloneToolsDetachAgentTemplate(_) => "standalone_tools/detach_agent_template",
            ClientRequest::DependencyToolsList(_) => "dependency_tools/list",
            ClientRequest::DependencyToolsGet(_) => "dependency_tools/get",
            ClientRequest::DependencyToolsDelete(_) => "dependency_tools/delete",
            ClientRequest::DependencyToolsTag(_) => "dependency_tools/tag",
            ClientRequest::DependencyToolsUntag(_) => "dependency_tools/untag",
            ClientRequest::DependencyToolsFiletree(_) => "dependency_tools/filetree",
            ClientRequest::DependencyToolsDownload(_) => "dependency_tools/download",
            ClientRequest::DependencyToolsUpload(_) => "dependency_tools/upload",
            ClientRequest::DependencyToolsTransfer(_) => "dependency_tools/transfer",
            ClientRequest::VaultGet(_) => "vault/get",
            ClientRequest::VaultInsert(_) => "vault/insert",
            ClientRequest::VaultDelete(_) => "vault/delete",
            ClientRequest::VaultList(_) => "vault/list",
            ClientRequest::VaultTag(_) => "vault/tag",
            ClientRequest::VaultUntag(_) => "vault/untag",
            ClientRequest::Invalid(_) => "",
        }
    }
}
