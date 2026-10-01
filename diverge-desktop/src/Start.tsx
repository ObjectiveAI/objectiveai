import { useEffect, useState } from "react";
import type { FirstRunView } from "./bindings/FirstRunView";
import { App } from "./App";
import { api } from "./lib/ipc";
import { FirstRun } from "./screens/FirstRun";

/** The first-run page until it's finished; the app after. */
export function Start() {
  const [firstRun, setFirstRun] = useState<FirstRunView | null>(null);
  useEffect(() => {
    api.firstRun().then(setFirstRun);
  }, []);
  if (!firstRun) return null;
  if (firstRun.state === "new" || firstRun.state === "earlier") return <FirstRun state={firstRun} onDone={setFirstRun} />;
  // Finished, or it can't be finished in this copy of the app: the app's banners say why.
  return <App />;
}
