// The host's identities: the credentials signed in under the root's
// `identities/`, each one subject at one issuer, listed. `issuers`
// lists the issuers an identity is signed in at; `subjects` the
// subjects at one of them. Each is one answer, read from the files as
// they are then; no token is ever answered.

export * as issuers from "./issuers/index.js";
export * as subjects from "./subjects/index.js";
