import { useEffect, useRef, type ReactElement } from "react";
import { registerIframe, unregisterIframe } from "./plugin-bridge";

interface PluginPaneProps {
  pluginName: string;
}

/**
 * Renders a sandboxed iframe pointing at the plugin's UI bundle via
 * the host viewer's custom `plugin://` URI scheme. Registers the
 * iframe with the postMessage bridge on mount, unregisters on
 * unmount. The bridge forwards Tauri events into the iframe.
 *
 * Inactive plugin panes are unmounted (not just hidden) so their
 * iframe memory is reclaimed; performance follow-up if hot-switching
 * between many plugins becomes a UX issue.
 */
export function PluginPane({ pluginName }: PluginPaneProps): ReactElement {
  const ref = useRef<HTMLIFrameElement | null>(null);

  useEffect(() => {
    const iframe = ref.current;
    if (!iframe) return;
    registerIframe(pluginName, iframe);
    return () => unregisterIframe(pluginName);
  }, [pluginName]);

  return (
    <iframe
      ref={ref}
      title={pluginName}
      src={`plugin://localhost/${encodeURIComponent(pluginName)}/index.html`}
      sandbox="allow-scripts allow-forms"
      style={{
        flex: 1,
        width: "100%",
        height: "100%",
        border: "none",
      }}
    />
  );
}
