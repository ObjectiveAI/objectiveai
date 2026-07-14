import { useMemo, type ReactElement } from "react";
import cn from "classnames";
import type { ViewerPluginInfo } from "./App";
import type { DaemonConnection } from "./lib/daemon";

interface PluginPaneProps {
  info: ViewerPluginInfo;
  /** The daemon coordinates, threaded down from App. Plugins talk to
   * the daemon THEMSELVES — the same SSE executor/listeners the
   * main viewer uses — so the coordinates ride the iframe URL as
   * query parameters (`daemon_address` / `daemon_signature`). */
  connection: DaemonConnection | null;
}

export function PluginPane({
  info,
  connection,
}: PluginPaneProps): ReactElement | null {
  const src = useMemo(() => {
    if (connection === null) return null;
    try {
      const url = new URL(info.iframe_src);
      url.searchParams.set("daemon_address", connection.address);
      if (connection.signature != null) {
        url.searchParams.set("daemon_signature", connection.signature);
      }
      return url.toString();
    } catch {
      // Unparseable src — mount it verbatim; the plugin just can't
      // reach the daemon.
      return info.iframe_src;
    }
  }, [info.iframe_src, connection]);

  // Mount once the daemon coordinates exist so the plugin's first load
  // already carries them (panes stay mounted for their whole life).
  if (src === null) return null;

  return (
    <iframe
      title={info.name}
      src={src}
      sandbox="allow-scripts allow-forms"
      className={cn("flex-1", "w-full", "h-full", "border-0")}
    />
  );
}
