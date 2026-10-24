// Naming an agent, a tool, a dependency tool or a volume: by its
// name, or once and for all.
//
// Every request that acts on an agent or a tool of the caller's
// names it one of two ways, the caller choosing which: by its NAME,
// if its create gave one, or by its TEMPLATE and its INDEX, which
// name it once and for all. A connected tool has no template, and is
// named by the DAEMON, the ACCOUNT and the TOOL it is registered
// from. A dependency tool is named by its template and its `Owner`;
// a volume one way only, by its provider and its name.

export * from "./agent.js";
export * from "./tool.js";
export * from "./dependency.js";
export * from "./owner.js";
export * from "./volume.js";
