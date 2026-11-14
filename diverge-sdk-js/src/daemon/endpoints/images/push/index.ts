// Pushing an image into the daemon's Store on a provider, without
// building it: the provider's `images.push`, asked of the daemon, which
// asks the provider as itself.
//
// 1. Into the daemon's Store: what is pushed the daemon owns there, as
//    it owns what a build builds, and an `images.list` shows it.
// 2. Where the image comes from is the provider's order: a digest
//    owned already is `exists`; a registry the provider uses that
//    serves the digest supplies it; otherwise the CLIENT supplies it.
// 3. The provider's asks are the client's to answer: a manifest, each
//    blob the provider lacks, and, last and always, the proof — each
//    opened by the daemon as a channel on the client's scope, as the
//    provider wrote it, and the answer carried back as the client
//    wrote it. The daemon supplies nothing of its own.
// 4. An ask the client does not answer is not held, and the push is
//    then `unavailable` or `proof_failed`.
// 5. Judged by provider: `push` over images, reaching the provider
//    named, by its identity or its tags.
// 6. A push cannot be stopped from the client's side.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
