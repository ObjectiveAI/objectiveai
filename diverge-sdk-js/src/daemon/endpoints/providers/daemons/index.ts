// Daemons: other daemons the caller holds accounts on, reached
// through a provider both are connected to.
//
// A daemon record is a name and its `Link`s, one per provider the
// remote is reachable through; on it are its `accounts`, each a name
// and a mode. What a record is FOR is acting as this daemon on that
// one, as one of its accounts: a connected tool, or a relayed request.

export * from "./account.js";
export * from "./link.js";
export * as accounts from "./accounts/index.js";
export * as add from "./add/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
