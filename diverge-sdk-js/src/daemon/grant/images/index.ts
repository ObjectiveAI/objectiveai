// Grants over images: the daemon's Stores, provider by provider.
//
// `{"images":{"actions":["build","push","list"],"within":"any"}}`, or
// `"within":{"identities":[…]}`, or `"within":{"all_tags":[…]}`.
//
// 1. Every action is judged by the provider: `build` and `push` by the
//    provider the request names, `list` by the provider each image is
//    on.
// 2. Nothing of the image is judged: not its digest, not when it came
//    to be owned.
// 3. The Store is the daemon's: every account reaching a provider
//    reaches the same Store there.
// 4. Reading is elsewhere: `images.read` is held by `read` over
//    providers and `relay` over daemons.

export * from "./permission.js";
export { Action } from "../../../shared/permission/images/index.js";
