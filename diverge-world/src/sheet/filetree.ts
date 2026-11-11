// Reading one entity's files: a container's tree, watched live, or a
// volume's, asked once.

import { daemon, shared } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";
import type { Files } from "../world/model";
import type { Following } from "./log";

/** The tree as it stands: the fold every frame goes into. */
export type Root = shared.filetree.response.Root;

/** A frame of a container's watch, as the three families answer it alike. */
type Watched = daemon.endpoints.agents.filetree.server.response.Frame;

/** Open a container's watch, by the family the entity is of, on the daemon `route` reaches. */
function open(files: Exclude<Files, { volume: unknown }>, route: Route) {
  if ("agent" in files) return daemon.endpoints.agents.filetree.client.execute(handle, { agent: files.agent, ...relayed(route) });
  if ("tool" in files) return daemon.endpoints.standalone_tools.filetree.client.execute(handle, { tool: files.tool, ...relayed(route) });
  return daemon.endpoints.dependency_tools.filetree.client.execute(handle, { dependency: files.dependency, ...relayed(route) });
}

/**
 * Read `files`, on the daemon `route` reaches.
 *
 * The rules:
 *
 * 1. A container's — an agent's, a tool's, a dependency tool's — is
 *    watched (its `filetree`): every frame is folded into one root, and
 *    `changed` is told the root and the paths the frame named
 *    (`inserted` and `modified`; a `snapshot` or a `removed` names
 *    none).
 * 2. A volume's is asked once (`volumes.filetree`, the whole volume),
 *    and its tree is the root, told to `changed` naming no path.
 * 3. `not_found`, `not_directory`, `held`, `forbidden`, an error, a
 *    watch that ends and a connection that fails each end the read,
 *    said once to `ended` in words. A volume's answer ends nothing.
 * 4. `stop` cancels a watch, at once if it is open and as soon as it
 *    opens if not; after it, nothing more is told.
 * 5. The watch and the ask are both served along `route` (`relayed`).
 */
export function readTree(files: Files, route: Route, changed: (root: Root, touched: string[][]) => void, ended: (why: string) => void): Following {
  let stopped = false;
  let cancel: { cancel(): Promise<void> } | null = null;
  const fail = (error: unknown) => {
    if (!stopped) ended(error instanceof Error ? error.message : String(error));
  };
  // Rule 2.
  if ("volume" in files) {
    void (async () => {
      try {
        const answer = await daemon.endpoints.volumes.filetree.client.execute(handle, { volume: files.volume, path: [], ...relayed(route) });
        if (stopped) return;
        if (typeof answer === "object" && "tree" in answer) return changed(new shared.filetree.response.Root(answer.tree), []);
        // Rule 3.
        if (answer === "not_found") return ended("the volume is not found");
        if (answer === "not_directory") return ended("the volume's root is not a directory");
        if (answer === "held") return ended("in use by a running container, or a transfer");
        if (answer === "forbidden") return ended("this account may not read the volume");
        ended("the daemon could not read the volume");
      } catch (error) {
        fail(error);
      }
    })();
    return { stop: () => void (stopped = true) };
  }
  void (async () => {
    try {
      // Rule 1.
      const [stream, c] = await open(files, route);
      cancel = c;
      if (stopped) {
        await c.cancel();
        return;
      }
      const root = new shared.filetree.response.Root();
      for await (const frame of stream as AsyncIterable<Watched>) {
        if (stopped) return;
        // Rule 3.
        if (frame === "not_found") return ended("dependency" in files ? "not running" : "not found");
        if (frame === "forbidden") return ended("this account may not read the files");
        if ("error" in frame) return ended("the daemon could not read the files");
        const f = frame.filetree;
        root.update(f);
        changed(root, f.type === "inserted" || f.type === "modified" ? [f.path] : []);
      }
      // The container stopped, which ends its watch: the tree stays as it last was.
      if (!stopped) ended("the container stopped — this is its tree as it last was");
    } catch (error) {
      fail(error);
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
