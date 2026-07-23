import { useTabHarness } from "../lib/tabHarness";
import { LaboratoriesPane } from "../components/LaboratoriesPane";

/** The laboratories home tab. */
export default function LaboratoriesTab() {
  const { transport } = useTabHarness();
  return <LaboratoriesPane transport={transport} />;
}
