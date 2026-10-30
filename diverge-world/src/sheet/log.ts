// Following one log, an agent's or a tool's: its history, then every
// item as it comes.

import { daemon } from "@diverge/sdk";
import { handle } from "../lib/wire";
import type { ToolRef } from "../world/model";

type AgentItem = daemon.endpoints.agents.logs.server.response.ItemWrapper;
type ToolItem = daemon.tool_log.ItemWrapper;

/** An agent, by its template and its index. */
export type AgentRef = { template: string; index: number };

/** A log being followed, until `stop`. */
export type Following = { stop(): void };

/** What every logs endpoint streams: an item, forbidden, or a failure. */
type Frame<T> = { item: T } | "forbidden" | { error: unknown };

/** A logs scope opened: its frames, and the cancel that ends it. */
type Opened<T> = readonly [AsyncIterable<Frame<T>>, { cancel(): Promise<void> }];

/**
 * Follow the log `open` opens.
 *
 * The rules:
 *
 * 1. The log is asked with `watch`: every item so far, in order, then
 *    every new one as it is kept, each handed to `item`.
 * 2. `forbidden`, an error, a stream that ends and a connection that
 *    fails each end the following, said once to `ended` in words.
 * 3. `stop` cancels the scope, at once if it is open and as soon as it
 *    opens if not; after it, nothing more is handed on and `ended` is
 *    not called.
 */
function follow<T>(open: () => Promise<Opened<T>>, item: (item: T) => void, ended: (why: string) => void): Following {
  let stopped = false;
  let cancel: { cancel(): Promise<void> } | null = null;
  void (async () => {
    try {
      // Rule 1.
      const [stream, c] = await open();
      cancel = c;
      if (stopped) {
        await c.cancel();
        return;
      }
      for await (const frame of stream) {
        if (stopped) return;
        // Rule 2.
        if (frame === "forbidden") return ended("this account may not read the log");
        if ("error" in frame) return ended("the daemon could not read the log");
        item(frame.item);
      }
      if (!stopped) ended("the log ended");
    } catch (error) {
      if (!stopped) ended(error instanceof Error ? error.message : String(error));
    }
  })();
  return {
    // Rule 3.
    stop() {
      stopped = true;
      void cancel?.cancel();
    },
  };
}

/** Follow `agent`'s log. */
export function followAgent(agent: AgentRef, item: (item: AgentItem) => void, ended: (why: string) => void): Following {
  return follow(() => daemon.endpoints.agents.logs.client.execute(handle, { agent, watch: true }), item, ended);
}

/** Follow `tool`'s log: a standalone tool's, or a dependency tool's. */
export function followTool(tool: ToolRef, item: (item: ToolItem) => void, ended: (why: string) => void): Following {
  return follow<ToolItem>(
    () =>
      tool.kind === "standalone"
        ? daemon.endpoints.standalone_tools.logs.client.execute(handle, { tool: tool.tool, watch: true })
        : daemon.endpoints.dependency_tools.logs.client.execute(handle, { dependency: tool.dependency, watch: true }),
    item,
    ended,
  );
}
