// Daemons: the other daemons of the configuration's, reached through
// a provider both are connected to.
//
// A remote daemon is a name, the issuer this daemon presents its
// identity from, and its `Link`s, one per provider it is reachable
// through. The remote holds this daemon's identity at that issuer as
// one of its accounts. What a remote is FOR is acting as this daemon
// on that one: a connected tool, or a relayed request. No request
// adds, edits or removes one.

export * from "./link.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
