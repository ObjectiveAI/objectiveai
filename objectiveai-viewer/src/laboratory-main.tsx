import React from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import LaboratoryApp from "./LaboratoryApp";
import "./app.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={300}>
      <LaboratoryApp />
    </TooltipProvider>
  </React.StrictMode>,
);
