// The FUSE vocabulary: what a mount asks of whoever serves it, and
// what it is answered.
//
// Nine asks, each one exchange on a channel of its own: `stat`,
// `read` and `list` look, each with an answer of its own; `write`,
// `truncate`, `setattr`, `remove`, `rename` and `mkdir` change
// something, and are each answered with an `ack`. Every frame is
// bytes, never JSON: a leading byte says which variant, and what
// follows is fixed-width integers, big-endian, and paths as UTF-8.
// The asks themselves are the container proxy's, under
// `container_proxy.outside.endpoints.fuse.mount`.

export * from "./error.js";
export * as bytes from "./bytes.js";
export * as prefixed from "./prefixed.js";
export * as time from "./time.js";
export * as entry from "./entry.js";
export * as ack from "./ack/index.js";
export * as stat from "./stat/index.js";
export * as read from "./read/index.js";
export * as list from "./list/index.js";
export * as setattr from "./setattr/index.js";
