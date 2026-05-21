import { useEffect, useRef, type ReactElement } from "react";
import cn from "classnames";
import type { ViewerPluginInfo } from "./App";
import { registerIframe, unregisterIframe } from "./plugin-bridge";

interface PluginPaneProps {
  info: ViewerPluginInfo;
}

export function PluginPane({ info }: PluginPaneProps): ReactElement {
  const ref = useRef<HTMLIFrameElement | null>(null);

  useEffect(() => {
    const iframe = ref.current;
    if (!iframe) return;
    registerIframe(info.name, iframe, info.iframe_src);
    return () => unregisterIframe(info.name);
  }, [info.name, info.iframe_src]);

  return (
    <iframe
      ref={ref}
      title={info.name}
      src={info.iframe_src}
      sandbox="allow-scripts allow-forms"
      className={cn("flex-1", "w-full", "h-full", "border-0")}
    />
  );
}
