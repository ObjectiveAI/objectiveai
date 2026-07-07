import React from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import App from "./App";
import { AgentWindow } from "./components/AgentWindow";
import "./function-tree/styles/function-tree.css";
import "./app.css";

/** Decode the hex the native agent-window builder puts in `?agent=`
 * (hex of the hierarchy's UTF-8 bytes) back to the hierarchy string.
 * Returns null on malformed input. */
function hexToString(hex: string): string | null {
  if (hex.length % 2 !== 0 || !/^[0-9a-fA-F]*$/.test(hex)) return null;
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return new TextDecoder().decode(bytes);
}

/** A native agent window loads the same bundle with `?agent=<hex>`;
 * render that agent's conversation instead of the main app. */
const agentHex = new URLSearchParams(window.location.search).get("agent");
const agentHierarchy = agentHex !== null ? hexToString(agentHex) : null;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={300}>
      {agentHierarchy !== null ? (
        <AgentWindow hierarchy={agentHierarchy} />
      ) : (
        <App />
      )}
    </TooltipProvider>
  </React.StrictMode>,
);
