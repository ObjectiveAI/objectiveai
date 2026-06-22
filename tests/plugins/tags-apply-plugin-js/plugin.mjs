// E2E test fixture: a plugin written in JavaScript, run by `plugins run` as
// `node ./plugin.mjs`. It uses the JS SDK's in-process PluginCommandExecutor
// + the generated `agents tags apply` execute fn to mutate host state (apply a
// tag to a mock agent) over the NDJSON command protocol, then emits one
// notification and exits.
//
// process.exit(0) is REQUIRED: the executor installs a readline listener on
// process.stdin, which keeps node's event loop alive — without the explicit
// exit, `plugins run` would block on the plugin's open stdout until the hang
// watchdog killed it.
import { PluginCommandExecutor, agentsTagsApplyExecute } from "@objectiveai/sdk";

const TAG = "js-plugin-applied-tag";

const executor = PluginCommandExecutor.instance();
await agentsTagsApplyExecute(executor, {
  name: TAG,
  target: { by: "agent", agent_spec: { upstream: "mock", output_mode: "instruction" } },
});

process.stdout.write(`${JSON.stringify({ type: "notification", applied: TAG })}\n`);
process.exit(0);
