// The page: empty up-front. Nothing is drawn yet; the daemon is
// reached through `lib/wire.ts` once there is something to show.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./main.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <main className="world" />
  </StrictMode>,
);
