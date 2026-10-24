// Grants over vault keys.
//
// `Permission` is one grant's worth over vault keys, reaching by the
// family's own list filter; the actions are `shared.permission`'s.

export * from "./permission.js";
export { Make, Over } from "../../../shared/permission/vault/index.js";
