// Events: the graph of who sent what to whom through the daemon.
//
// An EVENT is one edge of that graph: a sender, a receiver and an
// action. One event per sender, receiver and action, with a `count`
// that grows; an edge is never removed. `list` is the one request.

export * from "./action.js";
export * from "./event.js";
export * as list from "./list/index.js";
