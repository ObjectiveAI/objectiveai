// Running an agent: only what the provider streams back is here —
// the chunks of the agentic loop, which the daemon logs and a
// client reads out of `agents.logs`.

export * as server from "./server/index.js";
