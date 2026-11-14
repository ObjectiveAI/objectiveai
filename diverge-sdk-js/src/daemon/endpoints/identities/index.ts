// The host's identities: the credentials signed in under the root's
// `identities/`, each one subject at one issuer. `list` answers them,
// each with its tags, read from the files as they are then; `get`
// answers one of them, likewise; `tag` and
// `untag` change an identity's tags, which the daemon keeps; `login`
// signs one in on the daemon's host. No token is ever answered.

export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as login from "./login/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
