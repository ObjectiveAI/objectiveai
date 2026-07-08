import { useEffect, useState } from "react";
import cn from "classnames";
import { daemonConnection, type DaemonConnection } from "./lib/daemon";
import { ConversationView } from "./components/ConversationView";
import { ErrorToast } from "./components/ErrorToast";

/**
 * The agent conversation WINDOW — the `agent.html` entry. One AIH's
 * whole conversation, fullscreen: no tabs, no footer, just the
 * [`ConversationView`] (plus the error toast — failures stay loud).
 * The AIH arrives via the `aih` query parameter, set by the Rust
 * shell (`open_agent_window` / `--agent-instance-hierarchy`); the
 * window title is the AIH itself.
 */
function AgentApp() {
  const aih = new URLSearchParams(window.location.search).get("aih");
  const [connection, setConnection] = useState<DaemonConnection | null>(null);
  useEffect(() => {
    let cancelled = false;
    void daemonConnection().then((config) => {
      if (!cancelled && config !== null) setConnection(config);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (aih === null || aih === "") {
    return (
      <div
        className={cn(
          "h-screen",
          "flex",
          "items-center",
          "justify-center",
          "font-mono",
          "text-[11px]",
          "text-info-dim",
        )}
      >
        no agent — open this window with ?aih=&lt;agent instance hierarchy&gt;
      </div>
    );
  }

  return (
    <div className={cn("h-screen", "flex", "flex-col")}>
      <div className={cn("flex-1", "min-h-0")}>
        <ConversationView connection={connection} hierarchy={aih} />
      </div>
      <ErrorToast />
    </div>
  );
}

export default AgentApp;
