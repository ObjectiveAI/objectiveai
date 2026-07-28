import { type TabComponentProps } from "../lib/tabHarness";
import { CommandLogPane } from "../components/CommandLogPane";

/** One captured request's response items, by its broadcast id
 * (`arguments: { id }`). */
export default function CommandLogTab({ arguments: args }: TabComponentProps) {
  const { id } = (args ?? {}) as { id?: string };
  return <CommandLogPane id={id ?? ""} />;
}
