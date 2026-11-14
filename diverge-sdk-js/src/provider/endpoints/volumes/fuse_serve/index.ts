// A provider's FUSE serve of a volume, as far as the daemon's own
// serve reaches into it: the channels a caller opens on the served
// scope, their answers, and the handle that holds the scope — which a
// daemon's `volumes.fuse_serve` shares byte for byte.

export * as client from "./client/index.js";
export * as server from "./server/index.js";
