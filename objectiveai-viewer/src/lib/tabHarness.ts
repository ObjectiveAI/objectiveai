import { createContext, useContext } from "react";
import type { ViewerTransport } from "@objectiveai/sdk";

/** The props every tab component receives from the bootstrap — just
 * its opened `arguments`, verbatim. NOTE: `arguments` is a reserved
 * binding name in strict mode — destructure as
 * `{ arguments: args }`. */
export interface TabComponentProps {
  arguments?: unknown;
}

/** The shared services the bootstrap provides around every tab
 * component (built-in and, later, plugin alike): the daemon
 * transport and the chrome-driven canvas zoom. */
export interface TabHarness {
  transport: ViewerTransport | null;
  zoom: number;
}

const TabHarnessContext = createContext<TabHarness>({
  transport: null,
  zoom: 1,
});

export const TabHarnessProvider = TabHarnessContext.Provider;

/** The current tab's harness services. */
export function useTabHarness(): TabHarness {
  return useContext(TabHarnessContext);
}
