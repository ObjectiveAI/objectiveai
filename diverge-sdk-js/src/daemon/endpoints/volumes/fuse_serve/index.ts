// Serving a volume over FUSE, through the daemon: the provider's
// `volumes.fuse_serve`, asked of the daemon. A caller names a volume
// by its `reference.Volume` and the mode it means it to be in, and
// the daemon opens that provider's serve of it as itself. The first
// response answers whether it is served. Where it is, the scope stays
// open, and the caller opens on it the channels a provider's serve
// answers — the nine asks a FUSE mount makes, the file tree, and the
// cancel — which the daemon carries to the provider's serve and back,
// byte for byte, reading none of them.
//
// It is for a container reaching a volume that is not its own
// daemon's: a daemon whose agent or tool mounts from a volume on
// another daemon opens this scope there, relayed along a route where
// the volume's daemon is further off.
//
// 1. Its own grant. The caller holds `fuse_serve` over the volume, an
//    action no other implies: not `mount`.
// 2. Exactly one answer, first. After any but `serving` the finish
//    follows at once, and no channel is answered.
// 3. The volume is in use while it is served: a delete of it answers
//    `in_use`, and every request that takes a volume at rest answers
//    `held`.
// 4. The caller ends it: the finish follows the caller's cancel, the
//    caller gone, or the provider's serve ending.
// 5. A serve that ends before the caller cancelled is a serve lost,
//    and a caller that mounted from it ends the run that did.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
