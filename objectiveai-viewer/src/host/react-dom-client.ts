// Host-provided `react-dom/client` — see ./react.ts.
import ReactDOMClient from "react-dom/client";
export {
  createRoot,
  hydrateRoot,
  // @ts-expect-error -- real runtime export the types omit.
  version,
} from "react-dom/client";
export default ReactDOMClient;
