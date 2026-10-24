// Standalone tools, and their templates.
//
// A tool is an MCP server the daemon keeps for a caller: made from a
// template and run as a container, or registered from another
// daemon's. Agents reach it attached — directly, or by their
// template — and a client reaches it through a connect, a scope
// held open on which the MCP exchanges ride as channels.

export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as edit from "./edit/index.js";
export * as register from "./register/index.js";
export * as attach_agent_instance from "./attach_agent_instance/index.js";
export * as detach_agent_instance from "./detach_agent_instance/index.js";
export * as attach_agent_template from "./attach_agent_template/index.js";
export * as detach_agent_template from "./detach_agent_template/index.js";
export * as delete from "./delete/index.js";
export * as list from "./list/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * as download from "./download/index.js";
export * as upload from "./upload/index.js";
export * as transfer from "./transfer/index.js";
export * as filetree from "./filetree/index.js";
export * as connect from "./connect/index.js";
export * as templates from "./templates/index.js";
