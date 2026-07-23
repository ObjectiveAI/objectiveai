import { useTabHarness, type TabComponentProps } from "../lib/tabHarness";
import { LaboratoryBrowser } from "../components/LaboratoryBrowser";

/** One laboratory browser, by id + optional host pin
 * (`arguments: { id, machine?, machine_state? }`). */
export default function LaboratoryTab({ arguments: args }: TabComponentProps) {
  const { transport } = useTabHarness();
  const { id, machine, machine_state } = (args ?? {}) as {
    id?: string;
    machine?: string;
    machine_state?: string;
  };
  return (
    <LaboratoryBrowser
      transport={transport}
      id={id ?? ""}
      machine={machine}
      machineState={machine_state}
    />
  );
}
