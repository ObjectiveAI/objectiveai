// Who made a thing: the client, an agent, or a tool.
//
// Everything the daemon holds for a caller was made by somebody: by
// the client itself, over an endpoint, or by an agent or a tool of
// the client's, through the daemon. Every list item carries that one
// maker as its `creator`, a `Creator`. A creator is named once and
// for all, so that what it made still says so after the creator is
// deleted: an `Agent` by its template and its index, a `Tool` by its
// template and its index or, a dependency tool, by its template and
// whose it is, the `Client` by its identity. The name is carried
// beside, when the create gave one.

export * from "./creator.js";
export * from "./client.js";
export * from "./agent.js";
export * from "./tool.js";
