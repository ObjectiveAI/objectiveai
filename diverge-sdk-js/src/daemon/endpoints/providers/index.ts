// Providers: the ones the daemon's configuration names, and the
// daemons it reaches through them.
//
// A provider is an identity the daemon's configuration accepts — the
// `(issuer, subject)` its token establishes in the handshake — served
// the same whichever end dialled. No request adds, edits or removes
// one. A list or a get reports the daemon's `Status` with each and its
// latest capacity. A `daemons` entry is another daemon of the
// configuration's, reached through a provider both are connected to,
// where this daemon's identity is one of its accounts.

export * from "./status.js";
export * as daemons from "./daemons/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
