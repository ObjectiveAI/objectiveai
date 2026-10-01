import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./theme.css";
import "./app.css";
import { Start } from "./Start";

async function start() {
  // Outside the Tauri app (a plain browser): play back the preview snapshot.
  if (!("__TAURI_INTERNALS__" in window)) {
    const { installPreview } = await import("./preview/mock");
    installPreview();
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Start />
    </StrictMode>,
  );
}

start();
