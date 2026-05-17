import { useEffect, useRef, useState, type ReactElement } from "react";
import { registerIframe, unregisterIframe } from "./plugin-bridge";

interface PluginPaneProps {
  pluginName: string;
}

export function PluginPane({ pluginName }: PluginPaneProps): ReactElement {
  const ref = useRef<HTMLIFrameElement | null>(null);
  const [loadError, setLoadError] = useState(false);

  useEffect(() => {
    const iframe = ref.current;
    if (!iframe) return;
    registerIframe(pluginName, iframe);

    const handleError = () => setLoadError(true);
    iframe.addEventListener("error", handleError);

    const timeout = setTimeout(() => {
      try {
        if (iframe.contentDocument?.body?.childElementCount === 0) {
          setLoadError(true);
        }
      } catch {
        // cross-origin — plugin loaded something, that's fine
      }
    }, 5000);

    return () => {
      unregisterIframe(pluginName);
      iframe.removeEventListener("error", handleError);
      clearTimeout(timeout);
    };
  }, [pluginName]);

  if (loadError) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center max-w-sm">
          <div className="text-error text-sm font-semibold mb-2">Plugin failed to load</div>
          <div className="text-info-dim text-xs font-mono mb-4">{pluginName}</div>
          <button
            onClick={() => { setLoadError(false); }}
            className="px-3 py-1.5 rounded-sm text-xs font-mono bg-ground-surface text-info-mid hover:text-copper-bright border border-node-border transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

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
