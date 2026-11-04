// MCP as an agent: a scope held open on an agent, on which the
// client opens channels — one per MCP exchange, and the disconnect —
// and the daemon opens none. The daemon answers exactly one
// `connected`, and every exchange after is answered exactly as the
// same exchange from the agent's own container is: the agent's tools
// by the names it sees them by, stamped as the agent's, logged in
// each tool's log as the agent's. An exchange on an agent that is not
// up deploys it first, as a message would, and waits for it; a list,
// a call or a read keeps the agent active while it is in flight.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
