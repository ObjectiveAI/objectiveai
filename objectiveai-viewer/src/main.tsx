import React from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import App from "./App";
// @ts-expect-error CSS import has no type declaration
import "@objectiveai/function-tree/styles";
import "./app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={300}>
      <App />
    </TooltipProvider>
  </React.StrictMode>,
);
