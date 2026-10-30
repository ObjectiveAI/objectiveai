// Dependency tools: those an agent's program declared.
//
// A dependency tool is a dependency template under one owner — an
// agent, an agent template, or nobody, as the template's scope
// decides — made by the first registration that declared it there,
// never by a request. Its container runs with its agent's, or as a
// service the daemon keeps; the file operations find it running or
// answer `not_found`, and start nothing. A delete is of the template
// and every tool of it.

export * as list from "./list/index.js";
export * as logs from "./logs/index.js";
export * as get from "./get/index.js";
export * as delete from "./delete/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * as filetree from "./filetree/index.js";
export * as download from "./download/index.js";
export * as upload from "./upload/index.js";
export * as transfer from "./transfer/index.js";
