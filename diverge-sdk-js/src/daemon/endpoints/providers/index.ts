// Providers: the ones the daemon's configuration names.
//
// A provider is an identity the daemon's configuration accepts — the
// `(issuer, subject)` its token establishes in the handshake — served
// the same whichever end dialled. No request adds, edits or removes
// one. A list or a get reports the daemon's `Status` with each and its
// latest capacity. `discover` lists the identities discoverable through
// them: what a `daemons` record names a remote by.

export * from "./status.js";
export * as discover from "./discover/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
