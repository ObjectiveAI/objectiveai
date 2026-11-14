// Over images: the daemon's Stores, by the tags of their providers.
//
// An image is no record of the daemon's. What the daemon holds is a
// Store on each provider that keeps one, and that is what a grant over
// images reaches: an object of the `Action`s held and how far they
// reach, `{"images":{"actions":["build","list"],"within":"any"}}`.
// Here a grant names nobody, so `within` is `"any"` or the shared tags
// test over the providers' records' tags, as for `providers` and
// `providers_daemons`; the daemon's own grant may also name providers
// by identity: see `daemon.grant.images`.

export * from "./action.js";
export * from "./permission.js";
