// Providers: the records of the providers the daemon works with.
//
// A provider is a record, by the identity its token establishes in the
// handshake — the `(issuer, subject)` — with the `Direction` its
// connection is made in, what the daemon announces itself with there,
// whether it accepts other daemons and hears messages through it, and
// whether it is enabled. `add`, `edit`, `delete`, `enable`, `disable`,
// `tag` and `untag` change the records, each change taking effect at
// once; a delete forgets only a disabled provider nothing is pinned to.
// An edit changes what the daemon holds open on the connection as it
// stands, and closes the connection only when the record's `ips` no
// longer admit its source address. While a provider is connected the
// daemon holds open on it exactly what its record says, each
// independent of the other two: the provider's `daemons::announce`,
// with the record's `announcement`, when the record has one; its
// `daemons::accept` when `accept_daemons` is `true`; its
// `daemons::listen` when `messageable` is `true`. The daemon sends a
// message through a provider whatever the three are.
// A list or a get reports the daemon's `Status` with each and its latest
// capacity. `daemons` holds the requests over the daemons reached
// through them: list, listen and message.

export * from "./direction.js";
export * from "./ip.js";
export * from "./status.js";
export * as add from "./add/index.js";
export * as daemons from "./daemons/index.js";
export * as delete from "./delete/index.js";
export * as disable from "./disable/index.js";
export * as edit from "./edit/index.js";
export * as enable from "./enable/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
