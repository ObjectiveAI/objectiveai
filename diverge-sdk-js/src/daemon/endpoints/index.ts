// The daemon's endpoints, one folder each, in the daemon's own tree
// of families. Each endpoint is `client/request.ts`,
// `server/response.ts` and `client/execute.ts`, and is reached by
// importing it. What is here so far is the vocabulary the families
// share — the list filters every grant reaches by, the provider
// identity, the create request's pieces, the template types — each
// at its own family's path; the endpoints themselves come family by
// family.

export * as accounts from "./accounts/index.js";
export * as agents from "./agents/index.js";
export * as dependency_tools from "./dependency_tools/index.js";
export * as events from "./events/index.js";
export * as postgres from "./postgres/index.js";
export * as providers from "./providers/index.js";
export * as roles from "./roles/index.js";
export * as standalone_tools from "./standalone_tools/index.js";
export * as vault from "./vault/index.js";
export * as volumes from "./volumes/index.js";
