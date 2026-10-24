// What an agent or a tool is, once and for all, with its name
// beside: how one list item points at another.
//
// A name is optional and may be given again once its holder is
// deleted, so nothing that has to keep pointing at an agent or a
// tool points by name. An `Agent` is its template and its index; a
// `Tool` on record is its `Origin` and its index, and a dependency
// tool the template it runs from and its `Owner`; each carries its
// name beside, when it has one.

export * from "./agent.js";
export * from "./origin.js";
export * from "./owner.js";
export * from "./tool.js";
