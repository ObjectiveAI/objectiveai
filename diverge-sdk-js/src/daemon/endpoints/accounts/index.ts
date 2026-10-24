// Accounts: who a request is served for, and what they may do.
//
// An ACCOUNT is an identity — the `(issuer, subject)` a client's token
// establishes in the mutual handshake — that the daemon admits, with
// the roles it holds. Adding one says both: this identity may connect,
// and these are its permissions. A connection whose token names an
// identity that is no account's, and no provider's the daemon accepts,
// is closed in the handshake; a daemon with no account admits no
// client at all. An account is named by its identity, a `Reference`,
// one account per identity. No account holds a key, and no answer
// carries one: an identity's tokens are its issuer's.

export * from "./reference.js";
export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
