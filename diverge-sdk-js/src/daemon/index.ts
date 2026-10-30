// The daemon protocol.
//
// `client` holds the exchange shapes every endpoint's `execute` is
// built on; `endpoints` holds the endpoints, one folder each, in the
// tree the daemon's own module tree has. Around them the vocabulary
// every family is made of: who made a thing, `creator`; how one list
// item points at another, `key`; how a request names a record,
// `reference`; what a role allows, `grant`; how a record is edited,
// `patch`; what a container is made from, `template`, and what its
// create adds, `create`; where a transfer lands, `transfer`; what a
// download sends, `download`.

export * as client from "./client/index.js";
export * as create from "./create/index.js";
export * as creator from "./creator/index.js";
export * as download from "./download/index.js";
export * as endpoints from "./endpoints/index.js";
export * as grant from "./grant/index.js";
export * as key from "./key/index.js";
export * as patch from "./patch/index.js";
export * as reference from "./reference/index.js";
export * as template from "./template/index.js";
export * as tool_log from "./tool_log/index.js";
export * as transfer from "./transfer/index.js";

/**
 * The TCP port a daemon accepts the protocol on unless its own
 * configuration says otherwise: one above the port a provider
 * listens on, so that a daemon and a provider on one host do not
 * contend for it. What a client dials by default.
 */
export const DEFAULT_PORT = 14980;
