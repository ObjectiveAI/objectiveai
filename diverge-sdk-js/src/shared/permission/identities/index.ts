// Grants over the host's identities that name none of them.
//
// The identities are the files under the root's `identities/`, each
// one subject at one issuer, signed in on the host; the daemon keeps
// their tags. In the shapes `permission` states: `Make`, the action that
// brings one onto the host; `Over`, the actions over those there,
// reaching by tags; and `Permission`, one grant's worth of either, or of
// tagging. No token is ever answered.

export * from "./make.js";
export * from "./over.js";
export * from "./permission.js";
