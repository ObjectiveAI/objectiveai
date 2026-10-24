// Connecting to a tool: a scope held open on a created tool, on
// which the client opens channels — one per MCP exchange, and the
// disconnect — and the daemon opens none. The daemon starts the
// container if it does not run, holds it while the scope is open,
// answers exactly one `connected`, and the scope stays open until
// the disconnect, the tool's run ending, or the connection ending.
// What a client sends in a request's `_meta` reaches the tool as
// sent; what the tool answers is attested by the serving daemon.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
