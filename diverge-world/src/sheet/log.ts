// Following one log, an agent's or a tool's: its history, then every
// entry as it comes.

import { daemon } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";
import type { Subject } from "../world/model";

/** One entry of a log, in its subject's kind. */
export type Entry = daemon.endpoints.logs.server.response.Entry;

/** A log being followed, until `stop`. */
export type Following = { stop(): void };

/**
 * Follow `subject`'s log, on the daemon `route` reaches.
 *
 * The rules:
 *
 * 1. The log is asked with `watch` (the root `logs` read), of the
 *    daemon `route` reaches (`relayed`): every entry so far, in order,
 *    then every new one as it is kept, each handed to `entry`.
 * 2. `watch` — the history sent, the read now live — is told to
 *    `live`, once, and handed to nothing else.
 * 3. `forbidden`, an error, a stream that ends and a connection that
 *    fails each end the following, said once to `ended` in words.
 * 4. `stop` cancels the scope, at once if it is open and as soon as it
 *    opens if not; after it, nothing more is handed on and `ended` is
 *    not called.
 */
export function follow(subject: Subject, route: Route, entry: (entry: Entry) => void, ended: (why: string) => void, live: () => void): Following {
  let stopped = false;
  let cancel: { cancel(): Promise<void> } | null = null;
  void (async () => {
    try {
      // Rule 1.
      const [stream, c] = await daemon.endpoints.logs.client.execute(handle, { of: subject, watch: true, ...relayed(route) });
      cancel = c;
      if (stopped) {
        await c.cancel();
        return;
      }
      for await (const frame of stream) {
        if (stopped) return;
        // Rule 2.
        if (frame === "watch") {
          live();
          continue;
        }
        // Rule 3.
        if (frame === "forbidden") return ended("this account may not read the log");
        if ("error" in frame) return ended("the daemon could not read the log");
        entry(frame.item);
      }
      if (!stopped) ended("the log ended");
    } catch (error) {
      if (!stopped) ended(error instanceof Error ? error.message : String(error));
    }
  })();
  return {
    // Rule 4.
    stop() {
      stopped = true;
      void cancel?.cancel();
    },
  };
}
