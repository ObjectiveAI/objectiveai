// Asking an agent something: a chat message, or a structured
// decision, by the request's `type`. Both go to the agent in the
// order the daemon admitted them. Unwatched, the answer is the
// request's fate; watched, it is what the agent's log takes of it.
// One grant covers both: `message`, over agents.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
