// Outgoing providers: the ones the daemon dials.
//
// An outgoing provider is an address and a `Mode`: how the daemon
// authenticates to it. Its identity is the address.

export * from "./kind.js";
export * from "./mode.js";
export * as add from "./add/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
