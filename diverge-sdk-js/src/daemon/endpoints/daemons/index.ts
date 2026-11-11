// Remote daemons: the other daemons this daemon reaches, each record one
// account of ours on one of them, keyed by its `DaemonId` — the hash of
// what it reaches and as whom, so fixed but for its description and
// tags. There are no names: relays and connected tools name records by
// ID. Several records may name the same remote. A remote is reached
// through any connected provider whose `daemons.list` listing holds it by a
// route its record's `Routes` let through, one chosen at random, so it
// is reachable while it is announced so.

export * as add from "./add/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * from "./id.js";
export * from "./routes.js";
