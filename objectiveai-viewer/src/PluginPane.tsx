import { type ReactElement } from "react";
import cn from "classnames";
import type { ViewerPluginInfo } from "./App";

// TODO(plugins): plugin iframes lost their daemon access when the
// daemon_address/daemon_signature query params were removed — the
// webview no longer holds daemon coordinates (every daemon stream
// rides the Rust proxy's Tauri commands, which a sandboxed iframe
// cannot invoke). Plugins need a postMessage bridge to the host
// window or plugin-scoped proxy commands.

interface PluginPaneProps {
  info: ViewerPluginInfo;
}

export function PluginPane({ info }: PluginPaneProps): ReactElement | null {
  return (
    <iframe
      title={info.name}
      src={info.iframe_src}
      sandbox="allow-scripts allow-forms"
      className={cn("flex-1", "w-full", "h-full", "border-0")}
    />
  );
}
