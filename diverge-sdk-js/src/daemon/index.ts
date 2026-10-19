// The daemon protocol.
//
// `client` holds the exchange shapes every endpoint's `execute` is
// built on; `endpoints` holds the endpoints, one folder each, in the
// tree the daemon's own module tree has.

export * as client from "./client/index.js";
export * as endpoints from "./endpoints/index.js";

/**
 * The TCP port a daemon accepts the protocol on unless its own
 * configuration says otherwise: one above the port a provider
 * listens on, so that a daemon and a provider on one host do not
 * contend for it. What a client dials by default.
 */
export const DEFAULT_PORT = 14980;
