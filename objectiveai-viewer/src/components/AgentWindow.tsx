import { useEffect } from "react";
import cn from "classnames";
import { startDaemonListener } from "../daemon-listener";
import { registerActiveAgentsHandler } from "../hooks/useActiveAgents";
import { registerAgentCompletionsHandler } from "../listener-handlers/agentCompletions";
import { ConversationView } from "./ConversationModal";

/**
 * The root of a native agent window (a Tauri popout selected by
 * `main.tsx` on the `?agent=` query param). Renders one agent's
 * conversation full-bleed — no backdrop, close button, or Escape
 * handler, since the OS window chrome owns closing.
 *
 * Its own JS context means its own module singletons, so it runs the
 * same daemon init `App` does (all idempotent): the live completion
 * tail needs the listener + handlers, while historical logs load
 * regardless via `useAgent`'s own `agents/logs/list`.
 */
export function AgentWindow({ hierarchy }: { hierarchy: string }) {
  useEffect(() => {
    registerActiveAgentsHandler();
    registerAgentCompletionsHandler();
    startDaemonListener();
  }, []);

  return (
    <div className={cn("h-screen", "w-screen", "overflow-hidden", "bg-ground")}>
      <ConversationView hierarchy={hierarchy} />
    </div>
  );
}
