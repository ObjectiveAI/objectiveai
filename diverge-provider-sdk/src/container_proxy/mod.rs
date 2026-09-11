//! The wire between a provider server and the proxy inside a
//! container: one path for every request, a path per answer, and
//! one stream.
//!
//! Every container the provider runs — an AGENT container, whose
//! entrypoint runs an agentic loop on a prompt, and an MCP container,
//! whose entrypoint is an MCP connection — carries one proxy program
//! beside its own entrypoint. The proxy is how the container reaches
//! the caller's world and how the caller sees into the container,
//! and this module is what rides its WebSockets.
//!
//! Nothing in the container listens for the server but the proxy. It
//! listens for the server on [`OUTSIDE_PORT`], `14979`, and the
//! server dials every path on it — an agent container's loop
//! included, which the proxy forwards to the agent's own server
//! beside it:
//!
//! | path                                | carries |
//! |-------------------------------------|---------|
//! | `/requests`                         | every request the container makes; nothing comes back on it |
//! | `/mcp/list-tools/{channel}` and its three siblings | one MCP response, then the close |
//! | `/mcp/notifications/{channel}`      | notifications as they come, then the close |
//! | `/vault/get/{channel}` and its four siblings | one vault answer, then the close |
//! | `/fuse/mount`                       | the server sends one mount; the container answers once it is made, or why not, then the close |
//! | `/fuse/read/{channel}` and its six siblings | one answer for a mounted file's or directory's ask, then the close |
//! | `/command/{channel}`                | the command's items, then the close |
//! | `/postgres/{channel}`               | raw pgwire, both ways, until either side closes |
//! | `/filesystem/tree`                  | the server names what to leave out; filetree frames, or why there are none, sent by the container |
//! | `/filesystem/read`                  | the server names a file; the container answers its bytes, or why not, then the close |
//! | `/filesystem/write`                 | the server names a file and sends its content; the container answers ok or error |
//! | `/agent/register`                   | the server sends the agent, once; the container answers registered, or an error, then the close |
//! | `/agent/run`                        | the server sends the prompt; the container answers the loop's chunks, or an error, then the close |
//! | `/agent/schema`                     | the container answers its agent's JSON Schema, or an error, then the close |
//! | `/agent/enqueue`                    | the server sends a message for the loop's queue; the container answers its fate, when known, then the close |
//! | `/agent/dequeue`                    | the container answers whether the queue held anything, then the close |
//! | `/tool/list-tools` and its three siblings | the server sends MCP params; the container's own MCP server answers one result, or an error, then the close |
//! | `/tool/notifications`               | what the container's MCP server says on its own account, as it comes |
//!
//! The program inside the container has a listener of its own,
//! [`INSIDE_PORT`], `80`, on the loopback: `/mcp`, the agent's MCP
//! SERVER — the Streamable HTTP endpoint its MCP client speaks to,
//! every exchange one ask on `/requests` — `/vault/<op>`, the vault
//! as plain HTTP, and `/command`. Two listeners, one per audience, so
//! no path on either has to say which side it faces; those surfaces
//! are the program's, not wires of this module. One thing fits on
//! neither: the pgwire listener the container's database driver
//! dials is raw TCP, not HTTP, so it is its own loopback port —
//! [`postgres::LOOPBACK_PORT`]. And the five `/agent/*` paths are not
//! the proxy's to answer: each is one call to the agent container's
//! own HTTP server, on the loopback at [`agent::port()`], forwarded —
//! the proxy holds nothing of the loop's, and dials that server only
//! when the provider's server has opened a path that needs it, which
//! it does only on an agent container. See [`agent`] for the calls
//! and what their answers become.
//!
//! # A request is a frame; an answer is a WebSocket
//!
//! The container asks on [`/requests`](requests): one
//! [`Frame`](requests::request::Frame) per ask, carrying a CHANNEL the
//! container minted and the ask itself. The server answers by
//! opening a WebSocket at the ask's own path with that channel in
//! it, sending the answer as RAW messages — the response type and
//! nothing around it: no channel, no type byte, no finish — and then
//! CLOSING. The close is the end of the answer, which is why nothing
//! in a response message has to say so. An answer path opened and
//! closed cleanly with no message is the server saying the request
//! could not be served.
//!
//! The channel is a `u32` the container counts up, unique among its
//! requests not yet answered. A u32 rather than a small tag,
//! because a channel now names a WebSocket rather than a slot in a
//! table, and a database connection is one of them.
//!
//! # One shape per path, in this module
//!
//! A path is a module, and the tree of modules is the tree of
//! paths: [`filesystem`] holds `/filesystem/*`, [`agent`] holds
//! `/agent/*`, and every path module has the same shape:
//! `request/` holds the ASK and `response/` the ANSWER, whichever
//! side sends them. For the exchanges the container opens, the ask
//! is the payload that rides `/requests` as `request::Request` (and,
//! on postgres, the driver's bytes as `request::Frame`) and the
//! answer is what the server sends on the path as `response::Frame`.
//! For the paths the server opens — the three under [`filesystem`],
//! the five under [`agent`] and the five under [`tool`] — the ask is
//! the server's message, or nothing but the opening, and the answer
//! is the container's. A direction that carries
//! nothing has no folder; where a type is shared by several paths,
//! each path's folder re-exports it rather than defining it again.
//!
//! # The rules every path shares
//!
//! - Every message is one WebSocket BINARY frame. A text frame
//!   carries nothing this wire defines, and either party ignores
//!   one, as it ignores a ping.
//! - The SERVER dials. `/requests` accepts exactly one connection
//!   at a time, a second refused with `409` before the upgrade;
//!   the `/filesystem/*` paths accept as many as the server opens —
//!   each a subscription, each one file. An answer path
//!   is accepted for a channel the container announced and the
//!   server has not yet opened — an unknown channel is refused with
//!   `404`, a second opening with `409`.
//! - There is no handshake. The upgrade is it.
//! - A clean close — the Close frame, the handshake — is an answer
//!   complete or a session ended; an abrupt end is a connection
//!   dying, and the two are told apart.
//! - Nothing times anything out. A quiet connection is a connection
//!   still running.
//!
//! # What dying means, stated once
//!
//! `/requests` dying loses the requests not yet answered. What
//! happens to each is its kind's rule: the [`mcp`] exchanges are
//! asked again on the next connection, at-least-once accepted,
//! because the agent inside is waiting; a [`vault`] operation and a
//! [`command`] are reported to whoever asked as failed, because
//! neither is safe to repeat (a lock already held outlives the dead
//! connection by its TTL); a [`postgres`] announcement whose path
//! was never opened is a driver socket the proxy closes. An answer
//! path dying is that one request failing, by the same rule per
//! kind — and a postgres path dying is that session ending, the
//! driver's socket shut. The stream path ([`filesystem::tree`]) simply
//! starts over on the next connection, and a [`filesystem::read`] or
//! [`filesystem::write`] whose socket died is that one file failing,
//! nothing else, and nothing retries it. An `/agent/*` path dying is that one call to
//! the agent's server failing — a loop cut short, a fate never heard,
//! a registration whose answer never came — and nothing retries that
//! either: the call was made, and what it did is done. A
//! `/fuse/mount` dying is a mount whose fate the server did not hear,
//! and the run that asked for it does not go on.

pub mod agent;
pub mod command;
pub mod filesystem;
pub mod fuse;
pub mod mcp;
pub mod postgres;
pub mod requests;
pub mod tool;
pub mod vault;

/// The port the proxy listens for the SERVER on, inside the
/// container: every path of this module is dialed here, from
/// outside.
pub const OUTSIDE_PORT: u16 = 14979;

/// The port the proxy listens for the PROGRAM on, inside the
/// container, on the loopback only: `/mcp`, `/vault/<op>` and
/// `/command`, the surfaces the program beside the proxy dials.
pub const INSIDE_PORT: u16 = 80;
