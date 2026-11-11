// Naming an account.

import { Identity } from "../../../shared/identity.js";

/** Which account a request means: its identity, `{"issuer":…,"subject":…}`. One account per identity, so each names exactly one. */
export const Reference = Identity;
export type Reference = Identity;
