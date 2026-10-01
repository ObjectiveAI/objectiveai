import { useCallback, useEffect, useState } from "react";
import type { FirstRunView } from "./bindings/FirstRunView";
import { App } from "./App";
import { Button } from "./components/ui";
import { api, errorText } from "./lib/ipc";
import { FirstRun } from "./screens/FirstRun";
import { t } from "./strings";

/** The first-run page until it's finished; the app after. */
export function Start() {
  const [firstRun, setFirstRun] = useState<FirstRunView | null>(null);
  const [unknown, setUnknown] = useState<string | null>(null);
  const ask = useCallback(() => {
    setUnknown(null);
    api.firstRun().then(setFirstRun, (e) => setUnknown(errorText(e)));
  }, []);
  useEffect(ask, [ask]);
  if (unknown !== null) {
    // The app couldn't be asked: say so, with what it said, rather than a blank window.
    return (
      <div className="page">
        <div className="page-inner">
          <h1 className="page-title">{t.firstRun.title}</h1>
          <p className="warn">{t.firstRun.unknown} {unknown}</p>
          <div className="row-actions">
            <Button onClick={ask}>{t.firstRun.askAgain}</Button>
          </div>
        </div>
      </div>
    );
  }
  if (!firstRun) return null;
  if (firstRun.state === "new" || firstRun.state === "earlier") return <FirstRun state={firstRun} onDone={setFirstRun} />;
  // Finished, or it can't be finished in this copy of the app: the app's banners say why.
  return <App />;
}
