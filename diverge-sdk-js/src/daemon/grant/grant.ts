// One grant, by the kind it is over.

import { z } from "zod";
import * as events from "../../shared/permission/events/index.js";
import * as postgres from "../../shared/permission/postgres/index.js";
import * as accounts from "./accounts/index.js";
import * as agents from "./agents/index.js";
import * as agents_templates from "./agents_templates/index.js";
import * as dependency_tools from "./dependency_tools/index.js";
import * as providers_daemons from "./providers_daemons/index.js";
import * as providers_incoming from "./providers_incoming/index.js";
import * as providers_outgoing from "./providers_outgoing/index.js";
import * as roles from "./roles/index.js";
import * as standalone_tools from "./standalone_tools/index.js";
import * as standalone_tools_templates from "./standalone_tools_templates/index.js";
import * as vault from "./vault/index.js";
import * as volumes from "./volumes/index.js";

/**
 * One grant: a permission over one kind of thing the daemon holds.
 * Externally tagged by the kind's name — one object with one member,
 * `{"agents":…}`, `{"providers_incoming":…}` — whose value is that
 * kind's `Permission`, in one of the shapes `grant` states.
 */
export const Grant = z.union([
  z.strictObject({ agents: agents.Permission }),
  z.strictObject({ agents_templates: agents_templates.Permission }),
  z.strictObject({ standalone_tools: standalone_tools.Permission }),
  z.strictObject({ standalone_tools_templates: standalone_tools_templates.Permission }),
  z.strictObject({ dependency_tools: dependency_tools.Permission }),
  z.strictObject({ providers_outgoing: providers_outgoing.Permission }),
  z.strictObject({ providers_incoming: providers_incoming.Permission }),
  z.strictObject({ providers_daemons: providers_daemons.Permission }),
  z.strictObject({ accounts: accounts.Permission }),
  z.strictObject({ roles: roles.Permission }),
  z.strictObject({ volumes: volumes.Permission }),
  z.strictObject({ vault: vault.Permission }),
  z.strictObject({ postgres: postgres.Permission }),
  z.strictObject({ events: events.Permission }),
]);
export type Grant = z.infer<typeof Grant>;
