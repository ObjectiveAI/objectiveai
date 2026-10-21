// The container scopes of the provider protocol, as far as the
// daemon's answers reach into them: what an agent's run streams, and
// the caller half's channel readers, which a connect borrows.

export * as agents from "./agents/index.js";
export * as client from "./client/index.js";
