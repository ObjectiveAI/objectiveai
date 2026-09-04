//! Toolset definitions — Eliza's own capabilities, one switch each.
//!
//! Each switch names a PLUGIN the image carries and the harness
//! lists or skips when it constructs the runtime — the grain Eliza
//! itself switches things at. A toolset with nothing to configure
//! is an `Option<bool>` on [`Toolsets`], absent meaning off; one
//! that takes a credential
//! is an `Option` of its own structure, AUTH AS AN ARGUMENT exactly
//! as for the provider: absent is off, present is on with that
//! tool's key. Nothing about a tool's auth rides the filesystem or
//! a mount, and the request carries no environment for it to ride.
//!
//! Always on, no switch: `plugin-sql` (the database — required),
//! the basic-capabilities bundle inside core (REPLY, IGNORE, NONE,
//! CHOOSE_OPTION, ATTACHMENT, CALCULATE, CHANNEL_RECAP,
//! SEARCH_CHANNEL_TOPICS and its twenty-one providers — registered
//! unconditionally), `plugin-agent-skills` (skills are MOUNTS; the
//! switch is whether anything is mounted), and the harness's own
//! `diverge` plugin, which registers each of the caller's MCP tools
//! as a native action through the proxy — the tool channel.
//!
//! Always off, no switch, and the reason for each so nobody
//! re-opens it without a new fact:
//! - CONNECTORS (discord, telegram, x, slack, whatsapp, wechat,
//!   matrix, imessage, instagram, google-workspace): a second
//!   inbound channel the queue cannot see, and turns nobody drives.
//!   An agent that lives on X gets an X MCP server from its caller
//!   and posts by tool call, on the stream.
//! - CLOUD AND HOST (elizacloud, cloud-apps, app-control,
//!   app-manager, registry, native-filesystem, the `native-*` device
//!   bridges, capacitor, omarchy, companion): another product's
//!   plumbing.
//! - WALLETS (wallet, taskmarket): keys in a vault under a
//!   passphrase — a secret that does not rotate has no resource
//!   story, and a private key as a plain argument is not one to ship
//!   without a threat model.
//! - LIFE-OPS (personal-assistant, goals, todos, inbox, reminders,
//!   health, finances, blocker, feed): bound to connectors,
//!   proactivity and the host UI.
//! - SUB-AGENTS AND ORCHESTRATION (agent-orchestrator, workflow):
//!   background children outliving the run, off every stream it
//!   has — the delegation objection.
//! - LOCAL INFERENCE (local-inference, native-inference,
//!   native-llama, cli-inference): the caller names a model source;
//!   the container runs none.
//! - `pty` (a terminal service for the host app's web terminal),
//!   `scheduling` (a state-machine spine whose persistence the host
//!   injects — not a tool), `computeruse` (needs a display),
//!   `vision` (camera and on-device recognition; image description
//!   rides the provider), `notes` (a host UI view), `pdf` (its role
//!   beside the documents plugin is unread — a switch waits on a
//!   fact), trajectory-logger, form, video, fish-audio,
//!   local-storage, inmemorydb: diagnostics, device features, or
//!   alternatives to what the harness already decides.
//! - THIRD-PARTY PLUGINS, as a class: no install at start exists,
//!   and a drop-in mount would run unreviewed code in the agent's
//!   process with the tool channel in reach and no isolation. A
//!   plugin, in this project's lineage, is a container the caller
//!   runs — its tools arrive over MCP.

pub mod web_search;

mod toolsets;

pub use toolsets::*;
