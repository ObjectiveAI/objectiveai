import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./theme.css";
import "./app.css";
import { App } from "./App";
import { app } from "./lib/app";

async function start() {
  // The theme you picked goes on before anything is drawn, so the default never shows first.
  // None (or a file the app can't read, or no app at all in a plain browser) leaves the default.
  const theme = await app.theme().catch(() => null);
  if (theme) document.documentElement.dataset.theme = theme;
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

start();
