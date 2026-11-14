// Grants through providers, over the daemons connected to them.
//
// `Permission` is one grant's worth, reaching the providers a `Reach`
// lets through — by identity, by their records' tags, or both. The
// actions are `shared.permission`'s.

export * from "./permission.js";
export { Over } from "../../../shared/permission/providers_daemons/index.js";
