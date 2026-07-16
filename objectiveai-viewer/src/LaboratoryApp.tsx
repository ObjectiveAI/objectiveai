import { useEffect, useState } from "react";
import cn from "classnames";
import { viewerTransport } from "./lib/viewer-transport";
import type { ViewerTransport } from "@objectiveai/sdk";
import { LaboratoryBrowser } from "./components/LaboratoryBrowser";
import { ErrorToast } from "./components/ErrorToast";

/** The shell-injected laboratory identity (`open_laboratory_window`). */
type LaboratoryGlobal = {
  id: string;
  machine?: string | null;
  machineState?: string | null;
};

/**
 * The laboratory filesystem WINDOW — the `laboratory.html` entry. One
 * laboratory's live file tree, fullscreen ([`LaboratoryBrowser`] plus
 * the error toast). The laboratory identity — id and its optional
 * `(machine, machineState)` host pin — arrives via the shell-injected
 * global (`open_laboratory_window`), with query-param fallbacks
 * (`id`/`machine`/`machine_state`) for plain-browser dev; the window
 * title is the laboratory id.
 */
function LaboratoryApp() {
  // The Rust shell injects the identity as a pre-page-script global (a
  // URL query can't ride WebviewUrl::App — it's a path). The
  // query-param fallback keeps plain-browser dev workable.
  const injected = (window as { __LABORATORY__?: LaboratoryGlobal })
    .__LABORATORY__;
  const params = new URLSearchParams(window.location.search);
  const id = injected?.id ?? params.get("id");
  const machine = injected
    ? (injected.machine ?? undefined)
    : (params.get("machine") ?? undefined);
  const machineState = injected
    ? (injected.machineState ?? undefined)
    : (params.get("machine_state") ?? undefined);

  const [transport, setTransport] = useState<ViewerTransport | null>(null);
  useEffect(() => {
    let cancelled = false;
    void viewerTransport().then((t) => {
      if (!cancelled && t !== null) setTransport(t);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (id == null || id === "") {
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
        no laboratory — open this window with ?id=&lt;laboratory id&gt;
      </div>
    );
  }

  return (
    <div className={cn("h-screen", "flex", "flex-col")}>
      <LaboratoryBrowser
        transport={transport}
        id={id}
        machine={machine}
        machineState={machineState}
      />
      <ErrorToast />
    </div>
  );
}

export default LaboratoryApp;
