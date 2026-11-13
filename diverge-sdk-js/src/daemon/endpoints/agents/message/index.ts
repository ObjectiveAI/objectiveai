// Asking an agent something: a chat message, or a structured
// decision. Both are one question and one answer, and both go to the
// agent in the order the daemon admitted them. One grant covers both:
// `message`, over agents.

export * as chat from "./chat/index.js";
export * as structured_decisions from "./structured_decisions/index.js";
