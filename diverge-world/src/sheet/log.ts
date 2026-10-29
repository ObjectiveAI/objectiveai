// Following one agent's log: its history, then every item as it comes.

import { daemon } from "@diverge/sdk";
import { handle } from "../lib/wire";

type Item = daemon.endpoints.agents.logs.server.response.ItemWrapper;

const logs = daemon.endpoints.agents.logs;

/** An agent, by its template and its index. */
export type AgentRef = { template: string; index: number };

/** A log being followed, until `stop`. */
export type Following = { stop(): void };

/**
 * Follow `agent`'s log.
 *
 * The rules:
 *
 * 1. The log is asked with `watch`: every item so far, in order, then
 *    every new one as the agent makes it, each handed to `item`.
 * 2. `forbidden`, an error, a stream that ends and a connection that
 *    fails each end the following, said once to `ended` in words.
 * 3. `stop` cancels the scope, at once if it is open and as soon as it
 *    opens if not; after it, nothing more is handed on and `ended` is
 *    not called.
 */
export function follow(agent: AgentRef, item: (item: Item) => void, ended: (why: string) => void): Following {
  let stopped = false;
  let cancel: { cancel(): Promise<void> } | null = null;
  void (async () => {
    try {
      // Rule 1.
      const [stream, c] = await logs.client.execute(handle, { agent, watch: true });
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
