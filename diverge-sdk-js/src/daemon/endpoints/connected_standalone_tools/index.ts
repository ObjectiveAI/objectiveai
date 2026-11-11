// Connected standalone tools: another daemon's standalone tools, held
// here on their own record, under names, for this daemon's agents.
//
// A `create` names one by a record of that daemon, a route beyond
// it, and the tool as that daemon names it; the daemon reaches it
// over that daemon's `standalone_tools` `connect`, holding at most
// one connect per tool, opened at an agent's first use and let go
// when idle, and never runs, starts or stops it. What it is FOR is
// being attached to agents, as a standalone tool is. Every request
// names it by a `reference.ConnectedStandaloneTool`: its name, or the
// daemon, the tool there and its index. No request of the
// `standalone_tools` family finds a connected tool, and no request
// here finds a standalone one.

export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as edit from "./edit/index.js";
export * as delete from "./delete/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
export * as attach_agent_instance from "./attach_agent_instance/index.js";
export * as detach_agent_instance from "./detach_agent_instance/index.js";
export * as attach_agent_template from "./attach_agent_template/index.js";
export * as detach_agent_template from "./detach_agent_template/index.js";
