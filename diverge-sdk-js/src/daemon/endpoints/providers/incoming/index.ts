// Incoming providers: the credentials of the ones that dial the
// daemon.
//
// A provider that dials the daemon presents an unbrokered credential,
// and the daemon judges it by the credentials the client has added:
// each a key the daemon minted, naming the identity a peer presenting
// it has. One credential per identity; the key is answered once.

export * from "./credential.js";
export * as add from "./add/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
