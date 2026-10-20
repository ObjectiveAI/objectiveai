// Agents, and their templates.
//
// An agent is made from a template under a name, lives as a record
// the daemon keeps, and runs as a container on a provider from its
// first message. Its log is the daemon's; its container's files are
// reached through download, upload, transfer and a filetree watch.

export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as delete from "./delete/index.js";
export * as message from "./message/index.js";
export * as logs from "./logs/index.js";
export * as list from "./list/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * as download from "./download/index.js";
export * as upload from "./upload/index.js";
export * as transfer from "./transfer/index.js";
export * as filetree from "./filetree/index.js";
export * as templates from "./templates/index.js";
