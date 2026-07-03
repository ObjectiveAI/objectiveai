import { __privateAdd, __privateGet, __privateSet, __privateMethod } from './chunk-ZJKILUQ3.js';
import { z } from 'zod';

var JsonValueSchema = z.union([
  z.string(),
  z.number(),
  z.boolean(),
  z.null(),
  z.array(z.lazy(() => JsonValueSchema)),
  z.record(z.string(), z.lazy(() => JsonValueSchema))
]);

// src/viewer/event.ts
var ViewerEventSchema = z.union([z.object({
  destination: z.string(),
  sub_type: z.string(),
  type: z.literal("inbound"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (e.g. `daemon` for raw daemon-broadcast frames on\nthe `"objectiveai"` channel; the JS bridge repackages routed\n`plugins/run` frames as `plugins_run` for plugin iframes).').meta({ "variantTitle": "Inbound" }), z.object({
  destination: z.string(),
  type: z.literal("cli_command"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. One stdout JSONL line from an objectiveai cli\nbinary the host spawned for an `invokeCli` this iframe\nstarted, terminated by a synthetic `{"type":"end"}` line. No\nsub_type \u2014 a single invocation produces a single stream of\nlines.').meta({ "variantTitle": "CliCommand" })]).describe("Every event the viewer emits to the JS side. Serde-tagged on\n`type` so the JS bridge can pattern-match and decide how to\nrepackage each variant for the destination iframe.\n\n`destination` is `\"objectiveai\"` for built-in events, or the\nplugin's repository name otherwise. For `CliCommand` it's the\nrepository name of whichever iframe invoked the CLI \u2014 the bridge\nderives it from `MessageEvent.source`, the plugin author never\nsets it.").meta({ title: "viewer.Event" });

// src/cli/command/listenerExecution.ts
var CLI_COMMAND_LISTENER_EXECUTION_MODES = {
  "agents/enqueue": "unary",
  "agents/enqueue/request_schema": "unary",
  "agents/enqueue/response_schema": "unary",
  "agents/get": "unary",
  "agents/get/request_schema": "unary",
  "agents/get/response_schema": "unary",
  "agents/instances/get": "stream",
  "agents/instances/get/request_schema": "unary",
  "agents/instances/get/response_schema": "unary",
  "agents/instances/list": "stream",
  "agents/instances/list/request_schema": "unary",
  "agents/instances/list/response_schema": "unary",
  "agents/laboratories/attach": "unary",
  "agents/laboratories/attach/request_schema": "unary",
  "agents/laboratories/attach/response_schema": "unary",
  "agents/laboratories/detach": "unary",
  "agents/laboratories/detach/request_schema": "unary",
  "agents/laboratories/detach/response_schema": "unary",
  "agents/laboratories/list": "stream",
  "agents/laboratories/list/request_schema": "unary",
  "agents/laboratories/list/response_schema": "unary",
  "agents/list": "stream",
  "agents/list/request_schema": "unary",
  "agents/list/response_schema": "unary",
  "agents/logs/list": "stream",
  "agents/logs/list/request_schema": "unary",
  "agents/logs/list/response_schema": "unary",
  "agents/logs/open": "unary",
  "agents/logs/open/request_schema": "unary",
  "agents/logs/open/response_schema": "unary",
  "agents/logs/subscribe": "stream",
  "agents/logs/subscribe/request_schema": "unary",
  "agents/logs/subscribe/response_schema": "unary",
  "agents/logs/token-usage/get": "unary",
  "agents/logs/token-usage/get/request_schema": "unary",
  "agents/logs/token-usage/get/response_schema": "unary",
  "agents/logs/token-usage/subscribe": "stream",
  "agents/logs/token-usage/subscribe/request_schema": "unary",
  "agents/logs/token-usage/subscribe/response_schema": "unary",
  "agents/mcp/resources/list": "unary",
  "agents/mcp/resources/list/request_schema": "unary",
  "agents/mcp/resources/list/response_schema": "unary",
  "agents/mcp/resources/read": "unary",
  "agents/mcp/resources/read/request_schema": "unary",
  "agents/mcp/resources/read/response_schema": "unary",
  "agents/mcp/servers/list": "unary",
  "agents/mcp/servers/list/request_schema": "unary",
  "agents/mcp/servers/list/response_schema": "unary",
  "agents/mcp/tools/call": "unary",
  "agents/mcp/tools/call/request_schema": "unary",
  "agents/mcp/tools/call/response_schema": "unary",
  "agents/mcp/tools/list": "unary",
  "agents/mcp/tools/list/request_schema": "unary",
  "agents/mcp/tools/list/response_schema": "unary",
  "agents/message": "unary",
  "agents/message/request_schema": "unary",
  "agents/message/response_schema": "unary",
  "agents/publish": "unary",
  "agents/publish/request_schema": "unary",
  "agents/publish/response_schema": "unary",
  "agents/queue/delete": "unary",
  "agents/queue/delete/request_schema": "unary",
  "agents/queue/delete/response_schema": "unary",
  "agents/queue/deliver": "stream",
  "agents/queue/deliver/request_schema": "unary",
  "agents/queue/deliver/response_schema": "unary",
  "agents/queue/list": "stream",
  "agents/queue/list/request_schema": "unary",
  "agents/queue/list/response_schema": "unary",
  "agents/queue/open": "unary",
  "agents/queue/open/request_schema": "unary",
  "agents/queue/open/response_schema": "unary",
  "agents/spawn": "both",
  "agents/spawn/request_schema": "unary",
  "agents/spawn/response_schema": "unary",
  "agents/tags/apply": "unary",
  "agents/tags/apply/request_schema": "unary",
  "agents/tags/apply/response_schema": "unary",
  "agents/tags/lookup": "unary",
  "agents/tags/lookup/request_schema": "unary",
  "agents/tags/lookup/response_schema": "unary",
  "agents/wait": "unary",
  "agents/wait/request_schema": "unary",
  "agents/wait/response_schema": "unary",
  "api/config/address/get": "unary",
  "api/config/address/get/request_schema": "unary",
  "api/config/address/get/response_schema": "unary",
  "api/config/address/set": "unary",
  "api/config/address/set/request_schema": "unary",
  "api/config/address/set/response_schema": "unary",
  "api/config/backoff_max_elapsed_time_ms/get": "unary",
  "api/config/backoff_max_elapsed_time_ms/get/request_schema": "unary",
  "api/config/backoff_max_elapsed_time_ms/get/response_schema": "unary",
  "api/config/backoff_max_elapsed_time_ms/set": "unary",
  "api/config/backoff_max_elapsed_time_ms/set/request_schema": "unary",
  "api/config/backoff_max_elapsed_time_ms/set/response_schema": "unary",
  "api/config/commit_author_email/get": "unary",
  "api/config/commit_author_email/get/request_schema": "unary",
  "api/config/commit_author_email/get/response_schema": "unary",
  "api/config/commit_author_email/set": "unary",
  "api/config/commit_author_email/set/request_schema": "unary",
  "api/config/commit_author_email/set/response_schema": "unary",
  "api/config/commit_author_name/get": "unary",
  "api/config/commit_author_name/get/request_schema": "unary",
  "api/config/commit_author_name/get/response_schema": "unary",
  "api/config/commit_author_name/set": "unary",
  "api/config/commit_author_name/set/request_schema": "unary",
  "api/config/commit_author_name/set/response_schema": "unary",
  "api/config/get": "unary",
  "api/config/get/request_schema": "unary",
  "api/config/get/response_schema": "unary",
  "api/config/github_authorization/get": "unary",
  "api/config/github_authorization/get/request_schema": "unary",
  "api/config/github_authorization/get/response_schema": "unary",
  "api/config/github_authorization/set": "unary",
  "api/config/github_authorization/set/request_schema": "unary",
  "api/config/github_authorization/set/response_schema": "unary",
  "api/config/http_referer/get": "unary",
  "api/config/http_referer/get/request_schema": "unary",
  "api/config/http_referer/get/response_schema": "unary",
  "api/config/http_referer/set": "unary",
  "api/config/http_referer/set/request_schema": "unary",
  "api/config/http_referer/set/response_schema": "unary",
  "api/config/mcp_authorization/add": "unary",
  "api/config/mcp_authorization/add/request_schema": "unary",
  "api/config/mcp_authorization/add/response_schema": "unary",
  "api/config/mcp_authorization/del": "unary",
  "api/config/mcp_authorization/del/request_schema": "unary",
  "api/config/mcp_authorization/del/response_schema": "unary",
  "api/config/mcp_authorization/get": "unary",
  "api/config/mcp_authorization/get/request_schema": "unary",
  "api/config/mcp_authorization/get/response_schema": "unary",
  "api/config/mcp_timeout_ms/get": "unary",
  "api/config/mcp_timeout_ms/get/request_schema": "unary",
  "api/config/mcp_timeout_ms/get/response_schema": "unary",
  "api/config/mcp_timeout_ms/set": "unary",
  "api/config/mcp_timeout_ms/set/request_schema": "unary",
  "api/config/mcp_timeout_ms/set/response_schema": "unary",
  "api/config/objectiveai_authorization/get": "unary",
  "api/config/objectiveai_authorization/get/request_schema": "unary",
  "api/config/objectiveai_authorization/get/response_schema": "unary",
  "api/config/objectiveai_authorization/set": "unary",
  "api/config/objectiveai_authorization/set/request_schema": "unary",
  "api/config/objectiveai_authorization/set/response_schema": "unary",
  "api/config/openrouter_authorization/get": "unary",
  "api/config/openrouter_authorization/get/request_schema": "unary",
  "api/config/openrouter_authorization/get/response_schema": "unary",
  "api/config/openrouter_authorization/set": "unary",
  "api/config/openrouter_authorization/set/request_schema": "unary",
  "api/config/openrouter_authorization/set/response_schema": "unary",
  "api/config/user_agent/get": "unary",
  "api/config/user_agent/get/request_schema": "unary",
  "api/config/user_agent/get/response_schema": "unary",
  "api/config/user_agent/set": "unary",
  "api/config/user_agent/set/request_schema": "unary",
  "api/config/user_agent/set/response_schema": "unary",
  "api/config/x_title/get": "unary",
  "api/config/x_title/get/request_schema": "unary",
  "api/config/x_title/get/response_schema": "unary",
  "api/config/x_title/set": "unary",
  "api/config/x_title/set/request_schema": "unary",
  "api/config/x_title/set/response_schema": "unary",
  "api/kill": "unary",
  "api/kill/request_schema": "unary",
  "api/kill/response_schema": "unary",
  "api/spawn": "unary",
  "api/spawn/request_schema": "unary",
  "api/spawn/response_schema": "unary",
  "daemon/kill": "unary",
  "daemon/kill/request_schema": "unary",
  "daemon/kill/response_schema": "unary",
  "daemon/spawn": "stream",
  "daemon/spawn/request_schema": "unary",
  "daemon/spawn/response_schema": "unary",
  "db/config/address/get": "unary",
  "db/config/address/get/request_schema": "unary",
  "db/config/address/get/response_schema": "unary",
  "db/config/address/set": "unary",
  "db/config/address/set/request_schema": "unary",
  "db/config/address/set/response_schema": "unary",
  "db/config/database/get": "unary",
  "db/config/database/get/request_schema": "unary",
  "db/config/database/get/response_schema": "unary",
  "db/config/database/set": "unary",
  "db/config/database/set/request_schema": "unary",
  "db/config/database/set/response_schema": "unary",
  "db/config/get": "unary",
  "db/config/get/request_schema": "unary",
  "db/config/get/response_schema": "unary",
  "db/config/password/get": "unary",
  "db/config/password/get/request_schema": "unary",
  "db/config/password/get/response_schema": "unary",
  "db/config/password/set": "unary",
  "db/config/password/set/request_schema": "unary",
  "db/config/password/set/response_schema": "unary",
  "db/config/user/get": "unary",
  "db/config/user/get/request_schema": "unary",
  "db/config/user/get/response_schema": "unary",
  "db/config/user/set": "unary",
  "db/config/user/set/request_schema": "unary",
  "db/config/user/set/response_schema": "unary",
  "db/kill": "unary",
  "db/kill/request_schema": "unary",
  "db/kill/response_schema": "unary",
  "db/query": "unary",
  "db/query/request_schema": "unary",
  "db/query/response_schema": "unary",
  "db/spawn": "unary",
  "db/spawn/request_schema": "unary",
  "db/spawn/response_schema": "unary",
  "functions/execute/standard": "both",
  "functions/execute/standard/request_schema": "unary",
  "functions/execute/standard/response_schema": "unary",
  "functions/execute/swiss_system": "both",
  "functions/execute/swiss_system/request_schema": "unary",
  "functions/execute/swiss_system/response_schema": "unary",
  "functions/get": "unary",
  "functions/get/request_schema": "unary",
  "functions/get/response_schema": "unary",
  "functions/list": "stream",
  "functions/list/request_schema": "unary",
  "functions/list/response_schema": "unary",
  "functions/profiles/get": "unary",
  "functions/profiles/get/request_schema": "unary",
  "functions/profiles/get/response_schema": "unary",
  "functions/profiles/list": "stream",
  "functions/profiles/list/request_schema": "unary",
  "functions/profiles/list/response_schema": "unary",
  "functions/profiles/publish": "unary",
  "functions/profiles/publish/request_schema": "unary",
  "functions/profiles/publish/response_schema": "unary",
  "functions/publish": "unary",
  "functions/publish/request_schema": "unary",
  "functions/publish/response_schema": "unary",
  "kill-all": "unary",
  "kill-all/request_schema": "unary",
  "kill-all/response_schema": "unary",
  "laboratories/create": "unary",
  "laboratories/create/request_schema": "unary",
  "laboratories/create/response_schema": "unary",
  "laboratories/list": "stream",
  "laboratories/list/request_schema": "unary",
  "laboratories/list/response_schema": "unary",
  "mcp/config/address/get": "unary",
  "mcp/config/address/get/request_schema": "unary",
  "mcp/config/address/get/response_schema": "unary",
  "mcp/config/address/set": "unary",
  "mcp/config/address/set/request_schema": "unary",
  "mcp/config/address/set/response_schema": "unary",
  "mcp/config/get": "unary",
  "mcp/config/get/request_schema": "unary",
  "mcp/config/get/response_schema": "unary",
  "mcp/config/port/get": "unary",
  "mcp/config/port/get/request_schema": "unary",
  "mcp/config/port/get/response_schema": "unary",
  "mcp/config/port/set": "unary",
  "mcp/config/port/set/request_schema": "unary",
  "mcp/config/port/set/response_schema": "unary",
  "mcp/kill": "unary",
  "mcp/kill/request_schema": "unary",
  "mcp/kill/response_schema": "unary",
  "mcp/spawn": "unary",
  "mcp/spawn/request_schema": "unary",
  "mcp/spawn/response_schema": "unary",
  "plugins/get": "unary",
  "plugins/get/request_schema": "unary",
  "plugins/get/response_schema": "unary",
  "plugins/install/filesystem": "unary",
  "plugins/install/filesystem/request_schema": "unary",
  "plugins/install/filesystem/response_schema": "unary",
  "plugins/install/github": "unary",
  "plugins/install/github/request_schema": "unary",
  "plugins/install/github/response_schema": "unary",
  "plugins/list": "stream",
  "plugins/list/request_schema": "unary",
  "plugins/list/response_schema": "unary",
  "plugins/logs/list": "stream",
  "plugins/logs/list/request_schema": "unary",
  "plugins/logs/list/response_schema": "unary",
  "plugins/run": "stream",
  "plugins/run/request_schema": "unary",
  "plugins/run/response_schema": "unary",
  "python": "unary",
  "python/request_schema": "unary",
  "python/response_schema": "unary",
  "swarms/get": "unary",
  "swarms/get/request_schema": "unary",
  "swarms/get/response_schema": "unary",
  "swarms/list": "stream",
  "swarms/list/request_schema": "unary",
  "swarms/list/response_schema": "unary",
  "swarms/publish": "unary",
  "swarms/publish/request_schema": "unary",
  "swarms/publish/response_schema": "unary",
  "tools/get": "unary",
  "tools/get/request_schema": "unary",
  "tools/get/response_schema": "unary",
  "tools/install/filesystem": "unary",
  "tools/install/filesystem/request_schema": "unary",
  "tools/install/filesystem/response_schema": "unary",
  "tools/install/github": "unary",
  "tools/install/github/request_schema": "unary",
  "tools/install/github/response_schema": "unary",
  "tools/list": "stream",
  "tools/list/request_schema": "unary",
  "tools/list/response_schema": "unary",
  "tools/run": "stream",
  "tools/run/request_schema": "unary",
  "tools/run/response_schema": "unary",
  "update": "stream",
  "update/request_schema": "unary",
  "update/response_schema": "unary",
  "viewer/config/address/get": "unary",
  "viewer/config/address/get/request_schema": "unary",
  "viewer/config/address/get/response_schema": "unary",
  "viewer/config/address/set": "unary",
  "viewer/config/address/set/request_schema": "unary",
  "viewer/config/address/set/response_schema": "unary",
  "viewer/config/get": "unary",
  "viewer/config/get/request_schema": "unary",
  "viewer/config/get/response_schema": "unary",
  "viewer/config/secret/get": "unary",
  "viewer/config/secret/get/request_schema": "unary",
  "viewer/config/secret/get/response_schema": "unary",
  "viewer/config/secret/set": "unary",
  "viewer/config/secret/set/request_schema": "unary",
  "viewer/config/secret/set/response_schema": "unary",
  "viewer/config/signature/get": "unary",
  "viewer/config/signature/get/request_schema": "unary",
  "viewer/config/signature/get/response_schema": "unary",
  "viewer/config/signature/set": "unary",
  "viewer/config/signature/set/request_schema": "unary",
  "viewer/config/signature/set/response_schema": "unary",
  "viewer/generate_secret_signature_pair": "unary",
  "viewer/generate_secret_signature_pair/request_schema": "unary",
  "viewer/generate_secret_signature_pair/response_schema": "unary",
  "viewer/kill": "unary",
  "viewer/kill/request_schema": "unary",
  "viewer/kill/response_schema": "unary",
  "viewer/spawn": "unary",
  "viewer/spawn/request_schema": "unary",
  "viewer/spawn/response_schema": "unary"
};

// src/viewer/runListener.ts
var _items, _done, _waiters, _ResponseItemStream_instances, wake_fn;
var ResponseItemStream = class {
  constructor() {
    __privateAdd(this, _ResponseItemStream_instances);
    __privateAdd(this, _items, []);
    __privateAdd(this, _done, false);
    __privateAdd(this, _waiters, []);
  }
  /** Items received so far (live view; do not mutate). */
  get items() {
    return __privateGet(this, _items);
  }
  /** Whether the run's terminator has arrived. */
  get done() {
    return __privateGet(this, _done);
  }
  /** @internal — feed side. */
  _push(item) {
    if (__privateGet(this, _done)) return;
    __privateGet(this, _items).push(item);
    __privateMethod(this, _ResponseItemStream_instances, wake_fn).call(this);
  }
  /** @internal — feed side. */
  _end() {
    __privateSet(this, _done, true);
    __privateMethod(this, _ResponseItemStream_instances, wake_fn).call(this);
  }
  async *[Symbol.asyncIterator]() {
    let i = 0;
    for (; ; ) {
      while (i < __privateGet(this, _items).length) {
        yield __privateGet(this, _items)[i++];
      }
      if (__privateGet(this, _done)) return;
      await new Promise((resolve) => __privateGet(this, _waiters).push(resolve));
    }
  }
  /** Every item, resolved once the run's terminator arrives. */
  async toArray() {
    const out = [];
    for await (const item of this) out.push(item);
    return out;
  }
};
_items = new WeakMap();
_done = new WeakMap();
_waiters = new WeakMap();
_ResponseItemStream_instances = new WeakSet();
wake_fn = function() {
  const waiters = __privateGet(this, _waiters);
  __privateSet(this, _waiters, []);
  for (const wake of waiters) wake();
};
var RETAINED_RUNS_MAX = 256;
var _instance, _retained, _live, _skipped, _subscribers, _unlisten, _RunListener_instances, onFrame_fn, onRequest_fn, onResponse_fn, onEnd_fn;
var _RunListener = class _RunListener {
  constructor(options) {
    __privateAdd(this, _RunListener_instances);
    __privateAdd(this, _retained, []);
    __privateAdd(this, _live, /* @__PURE__ */ new Map());
    __privateAdd(this, _skipped, /* @__PURE__ */ new Set());
    __privateAdd(this, _subscribers, /* @__PURE__ */ new Set());
    __privateAdd(this, _unlisten, () => {
    });
    if (__privateGet(_RunListener, _instance)) {
      return __privateGet(_RunListener, _instance);
    }
    __privateSet(_RunListener, _instance, this);
    __privateSet(this, _unlisten, listen(
      options?.subType ?? "plugins_run",
      (frame) => {
        __privateMethod(this, _RunListener_instances, onFrame_fn).call(this, frame);
      }
    ));
  }
  /**
   * Iterate the runs: retained envelopes first (up to
   * {@link RETAINED_RUNS_MAX}), then live as they arrive. Multiple
   * iterators are independent; `return()`/`break` detaches cleanly.
   */
  runs() {
    const queue = [...__privateGet(this, _retained)];
    let wake = null;
    const push = (run) => {
      queue.push(run);
      wake?.();
    };
    __privateGet(this, _subscribers).add(push);
    const detach = () => __privateGet(this, _subscribers).delete(push);
    return {
      next: async () => {
        while (queue.length === 0) {
          await new Promise((resolve) => {
            wake = resolve;
          });
          wake = null;
        }
        return {
          value: queue.shift(),
          done: false
        };
      },
      return: async () => {
        detach();
        return { value: void 0, done: true };
      },
      throw: async (e) => {
        detach();
        throw e;
      },
      [Symbol.asyncIterator]() {
        return this;
      }
    };
  }
  [Symbol.asyncIterator]() {
    return this.runs();
  }
  /** Reset the singleton (tests only). */
  static __resetForTests() {
    var _a;
    const instance = __privateGet(_RunListener, _instance);
    if (instance) {
      __privateGet(_a = instance, _unlisten).call(_a);
    }
    __privateSet(_RunListener, _instance, void 0);
  }
};
_instance = new WeakMap();
_retained = new WeakMap();
_live = new WeakMap();
_skipped = new WeakMap();
_subscribers = new WeakMap();
_unlisten = new WeakMap();
_RunListener_instances = new WeakSet();
onFrame_fn = function(frame) {
  if (typeof frame !== "object" || frame === null) return;
  const f = frame;
  if (typeof f.id !== "string") return;
  if (f.end === true) {
    __privateMethod(this, _RunListener_instances, onEnd_fn).call(this, f.id);
  } else if (__privateGet(this, _live).has(f.id)) {
    __privateMethod(this, _RunListener_instances, onResponse_fn).call(this, f.id, f.value);
  } else if (__privateGet(this, _skipped).has(f.id)) ; else {
    __privateMethod(this, _RunListener_instances, onRequest_fn).call(this, f.id, frame);
  }
};
onRequest_fn = function(id, frame) {
  const request = frame["value"];
  if (typeof request !== "object" || request === null) {
    __privateGet(this, _skipped).add(id);
    return;
  }
  const req = request;
  if (typeof req.path_type !== "string") {
    __privateGet(this, _skipped).add(id);
    return;
  }
  const agentArguments = extractAgentArguments(frame);
  const transformed = req.jq != null || req.python != null;
  const mode = transformed ? "stream" : CLI_COMMAND_LISTENER_EXECUTION_MODES[req.path_type];
  if (mode === void 0) {
    __privateGet(this, _skipped).add(id);
    return;
  }
  const resolved = mode === "both" ? req.dangerous_advanced?.stream === true ? "stream" : "unary" : mode;
  let response;
  const feed = { pathType: req.path_type };
  if (resolved === "stream") {
    feed.stream = new ResponseItemStream();
    response = feed.stream;
  } else {
    response = new Promise((resolve) => {
      feed.resolve = resolve;
    });
  }
  __privateGet(this, _live).set(id, feed);
  const envelope = {
    request,
    agentArguments,
    response
  };
  __privateGet(this, _retained).push(envelope);
  if (__privateGet(this, _retained).length > RETAINED_RUNS_MAX) {
    __privateGet(this, _retained).shift();
  }
  for (const subscriber of [...__privateGet(this, _subscribers)]) {
    subscriber(envelope);
  }
};
onResponse_fn = function(id, value) {
  const feed = __privateGet(this, _live).get(id);
  if (!feed) return;
  if (feed.stream) {
    feed.stream._push(value);
  } else if (!feed.settled) {
    feed.settled = true;
    feed.resolve?.(value);
  }
};
onEnd_fn = function(id) {
  __privateGet(this, _skipped).delete(id);
  const feed = __privateGet(this, _live).get(id);
  if (!feed) return;
  __privateGet(this, _live).delete(id);
  if (feed.stream) {
    feed.stream._end();
  } else if (!feed.settled) {
    feed.settled = true;
    feed.resolve?.({
      type: "error",
      level: "error",
      fatal: null,
      message: `${feed.pathType}: run ended before any response item`
    });
  }
};
__privateAdd(_RunListener, _instance);
var RunListener = _RunListener;
function extractAgentArguments(frame) {
  const agentArguments = {};
  for (const key of [
    "agent_instance_hierarchy",
    "agent_id",
    "agent_full_id",
    "agent_remote",
    "response_id",
    "response_ids"
  ]) {
    const value = frame[key];
    if (typeof value === "string" || value === null) {
      agentArguments[key] = value;
    }
  }
  return agentArguments;
}

// src/viewer/index.ts
function isInIframe() {
  return typeof window !== "undefined" && window.parent !== window;
}
var inboundListeners = /* @__PURE__ */ new Map();
var inboundHandlerAttached = false;
function attachInboundHandler() {
  if (inboundHandlerAttached) return;
  if (typeof window === "undefined") return;
  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== "object") return;
    if (msg.kind !== "plugin-event") return;
    if (msg.type !== "inbound") return;
    const set = inboundListeners.get(msg.sub_type);
    if (!set) return;
    for (const fn of set) {
      try {
        fn(msg.value);
      } catch (e) {
        console.error("@objectiveai/sdk/viewer listener threw:", e);
      }
    }
  });
  inboundHandlerAttached = true;
}
function listen(sub_type, handler) {
  if (!isInIframe()) {
    let unlisten = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await import('./event-T244TCB6.js');
        if (cancelled) return;
        const u = await mod.listen(
          `plugin-${sub_type}`,
          (e) => handler(e.payload?.value)
        );
        if (cancelled) {
          u();
        } else {
          unlisten = u;
        }
      } catch {
        console.warn(
          `@objectiveai/sdk/viewer: listen('${sub_type}') called outside an iframe and @tauri-apps/api is unavailable; events will not fire.`
        );
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }
  attachInboundHandler();
  const set = inboundListeners.get(sub_type) ?? /* @__PURE__ */ new Set();
  const fn = (value) => handler(value);
  set.add(fn);
  inboundListeners.set(sub_type, set);
  return () => {
    const s = inboundListeners.get(sub_type);
    if (!s) return;
    s.delete(fn);
    if (s.size === 0) inboundListeners.delete(sub_type);
  };
}
function __resetForTests() {
  inboundListeners.clear();
}

export { CLI_COMMAND_LISTENER_EXECUTION_MODES, JsonValueSchema, ResponseItemStream, RunListener, ViewerEventSchema, __resetForTests, listen };
