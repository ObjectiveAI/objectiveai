//! Running ObjectiveAI CLI commands from inside a plugin.
//!
//! A plugin cannot run the CLI itself — there is no binary in its
//! container and no daemon it is allowed to dial. What it has is the
//! MCP connection the host already opened, and an extension that runs
//! backwards along it: the plugin declares the `objectiveai`
//! capability, pushes a `notifications/objectiveai/cli_request` frame
//! naming the command it wants, and the HOST runs it and POSTs the
//! results back, one frame per item, to
//! `{mcp_url}/objectiveai/command`.
//!
//! [`command_executor`] wraps that exchange in the SDK's own
//! [`CommandExecutor`] trait, so a plugin issues commands exactly the
//! way anything else in ObjectiveAI does.
//!
//! ```no_run
//! # use objectiveai_sdk::cli::command::CommandExecutor as _;
//! # async fn example() -> Result<(), objectiveai_mcp_plugin_framework::command_executor::Error> {
//! # let request: objectiveai_sdk::cli::command::update::Request = todo!();
//! let executor = objectiveai_mcp_plugin_framework::command_executor();
//! let _item: serde_json::Value = executor.execute_one(request, None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! **An executor can be taken before [`serve`][crate::serve::serve] is
//! called.** All of its state is process-global, so the value itself
//! carries nothing that has to exist yet. Used before a client has
//! connected, `execute` simply WAITS for one rather than failing:
//! there is no peer to push a notification to until then, and a plugin
//! that built its executor during startup should not be punished for
//! it.
//!
//! **Every exchange is independent.** Each run has its own correlation
//! id and its own channel, and the response endpoint routes purely by
//! id through a sharded map — so any number of commands run at once
//! and none of them queue behind another. Frame order is guaranteed
//! per run, never across runs.

use std::pin::Pin;
use std::sync::LazyLock;

use dashmap::DashMap;
use futures::{Stream, StreamExt as _};
use objectiveai_sdk::cli::command::{CommandRequest, CommandResponse};
use objectiveai_sdk::identity::Identity;
use objectiveai_sdk::mcp::CliResponse;
use rmcp::model::{CustomNotification, ServerNotification};
use rmcp::service::Peer;
use rmcp::RoleServer;
use tokio::sync::mpsc;

/// The notification a plugin pushes to ask the host to run a command.
const CLI_REQUEST_METHOD: &str = "notifications/objectiveai/cli_request";

/// Where the host POSTs the answering frames — the MCP endpoint plus
/// [`objectiveai_sdk::mcp::CLI_COMMAND_ENDPOINT_SUFFIX`]. `serve`
/// mounts it; it is here because this is the half that gives it
/// meaning.
pub(crate) const COMMAND_PATH: &str = objectiveai_sdk::mcp::CLI_COMMAND_ENDPOINT_SUFFIX;

/// Runs in flight, by correlation id.
///
/// A `DashMap` rather than one lock: the response endpoint touches it
/// on EVERY frame of EVERY run, and two runs answering at once must
/// not serialize against each other.
static PENDING: LazyLock<DashMap<String, mpsc::UnboundedSender<CliResponse>>> =
    LazyLock::new(DashMap::new);

/// The connected client, once one is. `serve` publishes it on
/// `initialize`; `execute` waits on it.
///
/// A `watch` rather than a slot plus a notify: a waiter that arrives
/// after the peer was published must not block forever waiting for a
/// signal that already fired, and `watch` is the channel whose
/// semantics are "the current value, and tell me when it changes".
static PEER: LazyLock<(
    tokio::sync::watch::Sender<Option<Peer<RoleServer>>>,
    tokio::sync::watch::Receiver<Option<Peer<RoleServer>>>,
)> = LazyLock::new(|| tokio::sync::watch::channel(None));

/// Publish the connected client. Called by `serve` on `initialize`.
pub(crate) fn set_peer(peer: Peer<RoleServer>) {
    let _ = PEER.0.send(Some(peer));
}

/// Route one answering frame to the run that is waiting for it.
///
/// Unknown ids are dropped silently: a run whose stream was abandoned
/// deregisters itself, and its late frames are not an error the host
/// should hear about.
pub(crate) fn deliver(frame: CliResponse) {
    let id = match &frame {
        CliResponse::Ack { id }
        | CliResponse::Item { id, .. }
        | CliResponse::Error { id, .. }
        | CliResponse::Done { id } => id.clone(),
    };
    if let Some(sink) = PENDING.get(&id) {
        let _ = sink.send(frame);
    }
}

/// Everything a command run can fail with.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The notification could not be pushed to the client.
    #[error("send the command request to the host")]
    Send(#[source] rmcp::service::ServiceError),
    /// A request or response item would not serialize.
    #[error("encode the command request")]
    Encode(#[source] serde_json::Error),
    /// A response item did not match the type the caller asked for.
    #[error("decode a command response item")]
    Decode(#[source] serde_json::Error),
    /// The CLI itself reported a failure for this item.
    #[error("{0}")]
    Cli(objectiveai_sdk::cli::Error),
    /// The HOST reported a failure — the run never started, or the
    /// stream broke on its side. Non-terminal on the wire: a run may
    /// report errors and keep going.
    #[error("the host could not run the command: {0}")]
    Host(String),
    /// The exchange ended before producing anything, and
    /// [`CommandExecutor::execute_one`] needed one item.
    #[error("the command produced no output")]
    Empty,
}

/// An executor that runs commands through the host.
///
/// A handle with nothing in it: every run reaches the same
/// process-global registry, so cloning is free and an instance taken
/// before the server started works exactly as well as one taken after.
#[derive(Debug, Clone, Copy, Default)]
pub struct Executor;

/// An executor for ObjectiveAI CLI commands, run by the host on this
/// plugin's behalf.
///
/// Safe to call at any point, including before
/// [`serve`][crate::serve::serve] — see the module docs.
pub fn command_executor() -> Executor {
    Executor
}

/// Deregisters a run when its stream is dropped, however it ends.
///
/// Without this an abandoned run would leak its slot, and — worse —
/// the id would stay live, so a late frame would be delivered to a
/// channel nobody reads.
struct Registered(String);

impl Drop for Registered {
    fn drop(&mut self) {
        PENDING.remove(&self.0);
    }
}

impl objectiveai_sdk::cli::command::CommandExecutor for Executor {
    type Error = Error;
    type Stream<T>
        = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        _identity: Option<&Identity>,
    ) -> Result<Self::Stream<T>, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        // `identity` is ignored, and cannot be otherwise: the HOST
        // decides who this plugin is — it stamps the trio from the
        // canonical image coordinates and refuses any claim off the
        // wire. A plugin asserting its own identity here would be
        // asserting nothing.
        let request = serde_json::to_value(&request).map_err(Error::Encode)?;
        let id = uuid::Uuid::new_v4().to_string();

        // REGISTER FIRST. The host may answer the instant the
        // notification lands, and a frame arriving before the slot
        // exists would be dropped as unknown.
        let (tx, rx) = mpsc::unbounded_channel();
        PENDING.insert(id.clone(), tx);
        let registered = Registered(id.clone());

        // Wait for a client if there is not one yet — see the module
        // docs. Cheap and immediate once connected.
        let peer = {
            let mut peer = PEER.1.clone();
            let connected = peer
                .wait_for(Option::is_some)
                .await
                .expect("the peer sender is a static and never drops");
            connected.clone().expect("waited for Some")
        };

        peer.send_notification(ServerNotification::CustomNotification(
            CustomNotification::new(
                CLI_REQUEST_METHOD,
                Some(serde_json::json!({ "id": id, "request": request })),
            ),
        ))
        .await
        .map_err(Error::Send)?;

        Ok(Box::pin(stream(rx, registered)))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        identity: Option<&Identity>,
    ) -> Result<T, Error>
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    {
        let mut stream = self.execute::<R, T>(request, identity).await?;
        stream.next().await.ok_or(Error::Empty)?
    }
}

/// Turn the frame channel into the typed item stream a caller sees.
///
/// The exchange is `Ack (Item|Error)* Done`. `Ack` says a response is
/// coming and carries nothing, so it is consumed silently; `Error` is
/// yielded and the run CONTINUES, matching the wire; `Done` ends the
/// stream. The channel closing also ends it, which is what a dropped
/// connection looks like from here.
fn stream<T>(
    rx: mpsc::UnboundedReceiver<CliResponse>,
    registered: Registered,
) -> impl Stream<Item = Result<T, Error>> + Send
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    futures::stream::unfold(
        (rx, Some(registered)),
        |(mut rx, registered)| async move {
            loop {
                let frame = rx.recv().await?;
                match frame {
                    // Nothing to hand a caller; it exists so the SERVER
                    // knows a slow run was picked up.
                    CliResponse::Ack { .. } => continue,
                    CliResponse::Item { item, .. } => {
                        return Some((decode(item), (rx, registered)));
                    }
                    CliResponse::Error { error, .. } => {
                        return Some((Err(Error::Host(error)), (rx, registered)));
                    }
                    // Terminal. Dropping `registered` here deregisters
                    // the run rather than waiting for the caller to
                    // drop the stream.
                    CliResponse::Done { .. } => return None,
                }
            }
        },
    )
    // `unfold` holds the guard in its state, so it is dropped whenever
    // the stream is — completed, cancelled or abandoned alike.
    .map(|item| item)
}

/// One wire item, as the type the caller asked for.
///
/// The frame hands over RAW JSON, so this decodes it ONCE, straight
/// into the leaf the caller named. It used to arrive as the untagged
/// `ResponseItem` sum and get re-encoded here — which meant the item
/// was first parsed as whichever of the ~400 leaves matched it, and
/// anything that guess did not model was lost before this ever ran
/// (see [`objectiveai_sdk::mcp::CliResponse::Item`]). A CLI-reported
/// failure rides the same shape, so it is decoded first.
fn decode<T>(value: serde_json::Value) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    if let Ok(error) = serde_json::from_value::<objectiveai_sdk::cli::Error>(value.clone()) {
        return Err(Error::Cli(error));
    }
    serde_json::from_value(value).map_err(Error::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path the host derives by appending the SDK's suffix to the
    /// MCP url — if these drift, the host POSTs into a 404 and every
    /// command hangs.
    #[test]
    fn the_command_path_is_the_sdk_suffix() {
        assert_eq!(COMMAND_PATH, "/objectiveai/command");
    }

    /// Delivery routes purely by id, and an unknown id is dropped
    /// rather than panicking — a run whose caller walked away must not
    /// be able to take the endpoint down.
    #[tokio::test]
    async fn frames_route_by_id_and_unknown_ids_are_dropped() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        PENDING.insert("known".to_string(), tx);

        deliver(CliResponse::Ack { id: "known".into() });
        deliver(CliResponse::Ack { id: "nobody-is-waiting".into() });

        assert!(matches!(rx.recv().await, Some(CliResponse::Ack { .. })));
        assert!(rx.try_recv().is_err(), "the stray frame went nowhere");
        PENDING.remove("known");
    }

    /// Dropping the stream deregisters the run, so a late frame has
    /// nowhere to land and the map does not grow.
    #[tokio::test]
    async fn dropping_the_stream_deregisters_the_run() {
        let (tx, rx) = mpsc::unbounded_channel();
        PENDING.insert("abandoned".to_string(), tx);
        let stream = stream::<serde_json::Value>(rx, Registered("abandoned".to_string()));

        assert!(PENDING.contains_key("abandoned"));
        drop(stream);
        assert!(!PENDING.contains_key("abandoned"));
    }

    /// `Ack` is consumed, `Item` is yielded, `Done` ends it — the
    /// whole exchange shape in one pass.
    #[tokio::test]
    async fn an_exchange_yields_only_its_items() {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = "exchange";
        PENDING.insert(id.to_string(), tx.clone());

        tx.send(CliResponse::Ack { id: id.into() }).unwrap();
        tx.send(CliResponse::Error {
            id: id.into(),
            error: "upstream said no".into(),
        })
        .unwrap();
        tx.send(CliResponse::Done { id: id.into() }).unwrap();

        let items: Vec<_> = stream::<serde_json::Value>(rx, Registered(id.to_string()))
            .collect()
            .await;
        assert_eq!(items.len(), 1, "the ack and the done are not items");
        assert!(matches!(items[0], Err(Error::Host(_))));
    }
}
