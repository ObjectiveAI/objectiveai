//! Driving the run: the whole lifetime, as one stream of chunks.
//!
//! [`run`] owns everything between the request and the end: the
//! lineage read and, when this run changes it, written
//! ([`Lineage`]); the fed secrets taken as this run's snapshot
//! ([`secrets`]); the caller's plugins, on both lists, installed
//! ([`plugins`]);
//! the agent rendered into settings ([`settings`]); the entry process
//! spawned and configured ([`Entry`]); the turns driven over its line
//! protocol ([`protocol`]) and converted into chunks ([`convert()`]);
//! the rotated secrets read back after every turn and left for the
//! proxy's collect; the queue consulted at each turn's end
//! ([`QUEUE`]); the entry stopped.
//!
//! # The continuation is the database
//!
//! Eliza's whole state is the caller's database, reached through the
//! proxy's pgwire by `POSTGRES_URL`: there is nothing to restore
//! before the runtime starts and nothing to harvest after it stops.
//! The one thing written before the runtime starts is the lineage
//! row, so a run that dies leaves a row naming the agent id its
//! memories carry.
//!
//! # The queue is consulted when a turn ends
//!
//! A turn is one `handleMessage`, and nothing steers it mid-way: a
//! message enqueued during a turn waits, and at the turn's end the
//! queue gets its look, atomically — an empty queue is closed in the
//! same lock hold that proved it empty, and the run ends; messages
//! pending are each answered `delivered`, yielded as a `user` chunk,
//! and, joined with a blank line between, become the next turn's
//! input in the same room. Invoking Eliza again IS the delivery.
//!
//! # What ends a run, and what does not
//!
//! Before the entry says `ready` there is nothing to salvage, and a
//! failure is the stream's one [`Err`] — the request's own failure, a
//! status. From then on every failure — a turn that fails, an entry
//! that dies, a read-back that will not answer — is a fatal
//! `notification` chunk, and the run still stops the entry where it
//! can: the database is the truth of what happened, consistent up to
//! the last committed write, and what was read back by then is what
//! the collect answers.
//!
//! # One run at a time, and the settlement releases the lock
//!
//! The run holds the [`Claim`], handed in by the server, inside a
//! [`Teardown`] captured into the stream: when the stream drops —
//! finished, or abandoned by a caller that left — the teardown marks
//! the claim settling, closes the queue on a task, and only then
//! drops the claim. The entry dies with the stream (`kill_on_drop`).

mod convert;
mod entry;
mod error;

pub mod protocol;

pub use convert::*;
pub use entry::*;
pub use error::*;

use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::{AgenticLoopChunk, user_parts};
use futures_util::Stream;
use rmcp::model::ContentBlock;
use sqlx::PgPool;

use crate::agent::{Agent, Plugin, Rotating};
use crate::agent::plugin::Rotates;
use crate::claim::Claim;
use crate::lineage::Lineage;
use crate::plugins;
use crate::queue::QUEUE;
use crate::secrets;
use crate::settings;
use protocol::{ReadKind, Request, Response};

/// The run, whole: one stream of chunks. `generation` is the queue's,
/// from [`QUEUE.open`](crate::queue::Queue::open); `claim` is the run
/// lock, released by the teardown.
pub fn run(
    client: Arc<Client>,
    pool: PgPool,
    agent: Agent,
    messages: Vec<Message>,
    generation: u64,
    claim: Claim,
) -> impl Stream<Item = Result<AgenticLoopChunk, Error>> {
    // Outside the generator, deliberately: a stream dropped before
    // its first poll never runs a line of the body, but its captured
    // locals still drop.
    let teardown = Teardown {
        claim: Some(claim),
        generation,
    };

    async_stream::stream! {
        let _teardown = teardown;

        let mut lineage = match Lineage::load(&pool).await {
            Ok(lineage) => lineage,
            Err(error) => {
                yield Err(Error::Lineage(error));
                return;
            }
        };

        // The secrets, as the proxy last fed them: this run's
        // snapshot, taken once, and what the collect answers until a
        // read-back finds something newer.
        let secrets = match secrets::values().await {
            Ok(secrets) => secrets,
            Err(error) => {
                yield Err(Error::Secrets(error));
                return;
            }
        };
        secrets::seed(&secrets).await;
        let rotating = agent.rotating();

        // Both lists, the model providers first, installed as one set
        // and recorded as one: the lineage pins every package alike.
        let listed: Vec<Plugin> = agent
            .model_provider_plugins
            .iter()
            .chain(&agent.plugins)
            .cloned()
            .collect();
        let resolved = match plugins::ensure(&listed, &lineage.plugins).await {
            Ok(resolved) => resolved,
            Err(error) => {
                yield Err(Error::Install(error));
                return;
            }
        };
        let providers = agent.model_provider_plugins.len();

        // The lineage, written BEFORE the runtime starts when this
        // run changes it.
        let mut notes: Vec<serde_json::Value> = Vec::new();
        if lineage.changed(&resolved) {
            if let Err(error) = lineage.save(&pool, resolved.clone()).await {
                yield Err(Error::Lineage(error));
                return;
            }
        }

        let rendered = settings::render(
            &agent,
            &secrets,
            client.postgres_url(),
            &lineage.vault_passphrase,
            &lineage.secret_salt,
        );

        let mut entry = match Entry::start(&rendered.env) {
            Ok(entry) => entry,
            Err(error) => {
                yield Err(Error::Entry(error));
                return;
            }
        };
        let configure = Request::Configure {
            agent_id: lineage.agent_id.clone(),
            character: rendered.character,
            settings: rendered.settings,
            model_provider_plugins: resolved[..providers]
                .iter()
                .map(|plugin| plugin.package.clone())
                .collect(),
            plugins: resolved[providers..]
                .iter()
                .map(|plugin| plugin.package.clone())
                .collect(),
            generate_media: rendered.generate_media,
            advanced_capabilities: rendered.advanced_capabilities,
            enable_relationships: rendered.enable_relationships,
            enable_documents: rendered.enable_documents,
            mcp_url: diverge_sdk::container_proxy::inside::mcp_url(),
        };
        if let Err(error) = entry.send(&configure).await {
            yield Err(Error::Entry(error));
            return;
        }
        loop {
            match entry.next().await {
                Ok(Some(Response::Ready)) => break,
                Ok(Some(Response::Fatal { error })) => {
                    yield Err(Error::Fatal(error));
                    return;
                }
                Ok(Some(Response::Notification { message })) => notes.push(message),
                Ok(Some(other)) => notes.push(serde_json::json!({
                    "kind": "protocol",
                    "error": "the entry wrote a line this program did not expect before ready",
                    "line": format!("{other:?}"),
                })),
                Ok(None) => {
                    yield Err(match entry.wait().await {
                        Ok(status) => Error::Exited(status),
                        Err(error) => Error::Entry(error),
                    });
                    return;
                }
                Err(error) => {
                    yield Err(Error::from(error));
                    return;
                }
            }
        }

        // From here on: chunks, and only chunks.
        for note in notes.drain(..) {
            yield Ok(notification(note, false));
        }

        // The rotating secrets a read-back ever found a value for.
        let mut found: BTreeSet<String> = BTreeSet::new();
        // Whether the entry still answers.
        let mut alive = true;
        // Whether the last turn's read-back is still owed.
        let mut owed = false;
        let content: Vec<ContentBlock> = messages.iter().flat_map(|message| message.content.iter().cloned()).collect();
        let mut input = crate::content::linked(content);
        let mut started_on = messages;

        'turns: loop {
            if let Err(error) = entry.send(&Request::Turn { content: input }).await {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "entry",
                        "error": format!("the turn could not be sent: {error}"),
                    }),
                    true,
                ));
                alive = false;
                break;
            }
            // The messages the run started on, as the stream's first
            // chunks: their parts, each under its key, before the
            // harness says a word. The turn is on the wire; a start
            // that failed was the request's own, above.
            for message in started_on.drain(..) {
                for chunk in user_parts(&message.key, message.content) {
                    yield Ok(chunk);
                }
            }
            owed = true;
            let mut spoke = false;
            loop {
                match entry.next().await {
                    Ok(Some(response)) => match convert(response) {
                        Converted::Chunk(chunk) => {
                            spoke |= matches!(chunk, AgenticLoopChunk::AssistantTextContent(_));
                            yield Ok(chunk);
                        }
                        Converted::Control(Response::Done { text, streamed, failure }) => {
                            // The final text is spoken only when no
                            // delta was: otherwise it repeats them.
                            if !(streamed || spoke) {
                                if let Some(text) = text.filter(|text| !text.is_empty()) {
                                    yield Ok(convert::text(text));
                                }
                            }
                            if let Some(failure) = failure {
                                yield Ok(notification(
                                    serde_json::json!({ "kind": "turn", "failure": failure }),
                                    true,
                                ));
                                break 'turns;
                            }
                            break;
                        }
                        Converted::Control(Response::Fatal { error }) => {
                            yield Ok(notification(
                                serde_json::json!({ "kind": "entry", "error": error }),
                                true,
                            ));
                            alive = false;
                            break 'turns;
                        }
                        Converted::Control(other) => yield Ok(unexpected(&other)),
                    },
                    Ok(None) => {
                        yield Ok(notification(
                            serde_json::json!({
                                "kind": "entry",
                                "error": "the entry exited mid-turn",
                            }),
                            true,
                        ));
                        alive = false;
                        break 'turns;
                    }
                    Err(LineError::Io(error)) => {
                        yield Ok(notification(
                            serde_json::json!({
                                "kind": "entry",
                                "error": format!("the entry's pipe failed mid-turn: {error}"),
                            }),
                            true,
                        ));
                        alive = false;
                        break 'turns;
                    }
                    Err(error) => {
                        yield Ok(notification(
                            serde_json::json!({ "kind": "protocol", "error": error.to_string() }),
                            false,
                        ));
                    }
                }
            }

            // The rotated secrets, read back and left for the collect
            // while the entry is quiet between turns.
            let mut chunks = Vec::new();
            let read = read_back(
                &mut entry,
                &rotating,
                &lineage.agent_id,
                &mut found,
                &mut chunks,
            )
            .await;
            for chunk in chunks.drain(..) {
                yield Ok(chunk);
            }
            owed = false;
            if let Err(error) = read {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "entry",
                        "error": format!("the read-back failed: {error}"),
                    }),
                    true,
                ));
                alive = false;
                break;
            }

            // The turn's end: the queue's look, atomic. Empty closes
            // it and ends the run; pending opens another turn.
            let taken = QUEUE.take_or_close().await;
            if taken.is_empty() {
                break;
            }
            let mut blocks = Vec::new();
            for message in taken {
                for chunk in user_parts(&message.key, message.content.clone()) {
                    yield Ok(chunk);
                }
                blocks.extend(message.content.clone());
                message.deliver();
            }
            input = crate::content::linked(blocks);
        }

        // A turn that ended in failure still owes its read-back, if
        // the entry can answer one.
        if alive && owed {
            let mut chunks = Vec::new();
            let read = read_back(
                &mut entry,
                &rotating,
                &lineage.agent_id,
                &mut found,
                &mut chunks,
            )
            .await;
            for chunk in chunks.drain(..) {
                yield Ok(chunk);
            }
            if let Err(error) = read {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "entry",
                        "error": format!("the read-back failed: {error}"),
                    }),
                    true,
                ));
                alive = false;
            }
        }

        // The stop: the runtime's ordinary stop, the database closed,
        // then the exit — awaited however long it takes.
        if alive {
            match entry.send(&Request::Stop).await {
                Ok(()) => loop {
                    match entry.next().await {
                        Ok(Some(Response::Stopped)) | Ok(None) => break,
                        Ok(Some(response)) => match convert(response) {
                            Converted::Chunk(chunk) => yield Ok(chunk),
                            Converted::Control(other) => yield Ok(unexpected(&other)),
                        },
                        Err(LineError::Io(_)) => break,
                        Err(error) => {
                            yield Ok(notification(
                                serde_json::json!({ "kind": "protocol", "error": error.to_string() }),
                                false,
                            ));
                        }
                    }
                },
                Err(error) => {
                    yield Ok(notification(
                        serde_json::json!({
                            "kind": "entry",
                            "error": format!("the stop could not be sent: {error}"),
                        }),
                        true,
                    ));
                    alive = false;
                }
            }
        }
        let exit = if alive { entry.wait().await } else { entry.kill().await };
        match exit {
            Ok(status) if status.success() || !alive => {}
            Ok(status) => {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "entry",
                        "error": format!("the entry exited with {status}"),
                    }),
                    true,
                ));
            }
            Err(error) => {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "entry",
                        "error": format!("the entry could not be reaped: {error}"),
                    }),
                    false,
                ));
            }
        }

        // A rotating secret nothing was ever read back for: the
        // collect answers the value it was fed, and the caller should
        // know before the next login fails.
        for secret in &rotating {
            if !found.contains(&secret.name) {
                yield Ok(notification(
                    serde_json::json!({
                        "kind": "read_back",
                        "key": secret.name,
                        "error": "nothing was found to read back; the collect answers the value fed",
                    }),
                    false,
                ));
            }
        }
    }
}

/// Read every rotating secret back from where its plugin leaves it,
/// and leave the ones that changed for the collect. A value found
/// marks the secret found; nothing found is nothing new; a channel
/// that fails is said, non-fatal, and the fed value stands. The
/// entry's stray lines while a read is pending — post-turn usage,
/// above all — are collected into `chunks` for the caller to yield.
/// Only the entry's pipe failing is an error.
async fn read_back(
    entry: &mut Entry,
    rotating: &[Rotating],
    agent_id: &str,
    found: &mut BTreeSet<String>,
    chunks: &mut Vec<AgenticLoopChunk>,
) -> Result<(), io::Error> {
    for secret in rotating {
        let value = match &secret.rotates {
            Rotates::InPlace(_) => ask(entry, ReadKind::Setting, &secret.name, chunks).await?,
            Rotates::Setting { setting } => ask(entry, ReadKind::Setting, setting, chunks).await?,
            Rotates::Vault { vault } => {
                let key = vault.replace("{agent_id}", agent_id);
                ask(entry, ReadKind::Vault, &key, chunks).await?
            }
            Rotates::File { file } => {
                let mut path = PathBuf::from("/");
                path.extend(file);
                match tokio::fs::read(&path).await {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(text) => Some(text),
                        // A secret is text; a file that is not
                        // cannot be handed back as one, and the fed
                        // value stands.
                        Err(_) => {
                            chunks.push(notification(
                                serde_json::json!({
                                    "kind": "read_back",
                                    "key": secret.name,
                                    "error": format!("{}: not UTF-8 text", path.display()),
                                }),
                                false,
                            ));
                            None
                        }
                    },
                    Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                    Err(error) => {
                        chunks.push(notification(
                            serde_json::json!({
                                "kind": "read_back",
                                "key": secret.name,
                                "error": format!("{}: {error}", path.display()),
                            }),
                            false,
                        ));
                        None
                    }
                }
            }
        };
        if let Some(value) = value {
            found.insert(secret.name.clone());
            secrets::leave(&secret.name, value).await;
        }
    }
    Ok(())
}

/// One read through the entry: the request, then lines until the
/// answer for that key, every other line sorted into `chunks`.
async fn ask(
    entry: &mut Entry,
    kind: ReadKind,
    key: &str,
    chunks: &mut Vec<AgenticLoopChunk>,
) -> Result<Option<String>, io::Error> {
    entry
        .send(&Request::Read {
            kind,
            key: key.to_string(),
        })
        .await?;
    loop {
        match entry.next().await {
            Ok(None) => {
                return Err(io::Error::other("the entry exited before answering a read"));
            }
            Ok(Some(Response::Answer { key: answered, value })) if answered == key => {
                return Ok(value);
            }
            Ok(Some(response)) => match convert(response) {
                Converted::Chunk(chunk) => chunks.push(chunk),
                Converted::Control(other) => chunks.push(unexpected(&other)),
            },
            Err(LineError::Io(error)) => return Err(error),
            Err(error) => chunks.push(notification(
                serde_json::json!({ "kind": "protocol", "error": error.to_string() }),
                false,
            )),
        }
    }
}

/// Settles the run when the stream drops, however it drops — and
/// THEN releases the run lock.
///
/// [`Drop`] cannot await, so the queue's closing rides a spawned
/// task; the graceful path makes it a no-op, and the close carries
/// the run's generation. The [`Claim`] rides with it and drops on
/// that task's next line: the next run can only open once this one's
/// queue is closed, and a request that lands meanwhile waits for it
/// rather than being refused, because the claim is marked settling
/// before the task is spawned.
struct Teardown {
    claim: Option<Claim>,
    generation: u64,
}

impl Drop for Teardown {
    fn drop(&mut self) {
        let claim = self.claim.take();
        if let Some(claim) = &claim {
            claim.settling();
        }
        let generation = self.generation;
        tokio::spawn(async move {
            QUEUE.close(generation).await;
            drop(claim);
        });
    }
}
