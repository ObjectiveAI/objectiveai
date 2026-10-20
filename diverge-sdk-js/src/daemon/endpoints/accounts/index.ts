// Accounts: who a request is served for, and what they may do.
//
// An ACCOUNT is the one principal the daemon knows. Every request is
// served for exactly one account and judged by that account's roles.
// An account is defined by a `Definition`: a name, a `Credential`, or
// both, and never neither; it is named afterwards by a `Reference`.
// The key is answered once, at the create or the edit that made the
// credential, and reported nowhere after.

export * from "./credential.js";
export * from "./definition.js";
export * from "./reference.js";
export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
