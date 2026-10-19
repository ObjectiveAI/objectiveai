import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./theme.css";
import "./app.css";
import { Start } from "./Start";
import { api } from "./lib/ipc";

async function start() {
  // Outside the Tauri app (a plain browser): play back the preview snapshot.
  if (!("__TAURI_INTERNALS__" in window)) {
    const { installPreview } = await import("./preview/mock");
    installPreview();
  }
  // The theme you picked goes on before anything is drawn, so the default never shows first.
  // None (or a file the app can't read) leaves the default.
  const theme = await api.theme().catch(() => null);
  if (theme) document.documentElement.dataset.theme = theme;
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Start />
    </StrictMode>,
  );
}

start();
