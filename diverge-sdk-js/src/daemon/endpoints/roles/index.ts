// Roles: named lists of grants, held by accounts.
//
// A ROLE is a name and a list of grants, each a permission over one
// kind of thing the daemon holds. An account holds zero or more
// roles, and may do what any grant of any role it holds allows: the
// union, with nothing that denies. One role per name, and the name
// is what every request names the role by.

export * as create from "./create/index.js";
export * as get from "./get/index.js";
export * as list from "./list/index.js";
export * as delete from "./delete/index.js";
export * as edit from "./edit/index.js";
export * as tag from "./tag/index.js";
export * as untag from "./untag/index.js";
